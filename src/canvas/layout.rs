use super::*;
use crate::Color;
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum Rotation {
    None,
    Left,
    Right,
    UpsideDown,
}

impl Default for Rotation {
    fn default() -> Self {
        Self::None
    }
}

impl std::ops::Neg for Rotation {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            Self::None => Self::None,
            Self::UpsideDown => Self::UpsideDown,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

impl Rotation {
    pub fn translate(self, x: i32, y: i32) -> (i32, i32) {
        match self {
            Self::None => (x, y),
            Self::UpsideDown => (-x, -y),
            Self::Left => (-y, x),
            Self::Right => (y, -x),
        }
    }
}

struct LayoutDevice<'a> {
    canvas: Box<dyn Canvas + 'a>,
    rotation: Rotation,
    x: u32,
    y: u32,
}

fn to_local(x: u32, y: u32, rot: Rotation, x_offset: u32, y_offset: u32) -> (u32, u32) {
    let x = x as i32;
    let y = y as i32;

    let (x, y) = (-rot).translate(x - x_offset as i32, y - y_offset as i32);

    (x as u32, y as u32)
}

fn to_global(x: u32, y: u32, rot: Rotation, x_offset: u32, y_offset: u32) -> (u32, u32) {
    let x = x as i32;
    let y = y as i32;

    let (x, y) = rot.translate(x, y);
    let (x, y) = (x + x_offset as i32, y + y_offset as i32);

    (x as u32, y as u32)
}

impl LayoutDevice<'_> {
    fn to_local(&self, x: u32, y: u32) -> (u32, u32) {
        to_local(x, y, self.rotation, self.x, self.y)
    }

    // not needed rn
    // fn to_global(&self, x: u32, y: u32) -> (u32, u32) {
    //     to_global(x, y, self.rotation, self.x, self.y)
    // }
}

/// Utility to be able to process messages from a CanvasLayout by polling
pub struct CanvasLayoutPoller {
    receiver: std::sync::mpsc::Receiver<CanvasMessage>,
}

impl crate::MsgPollingWrapper for CanvasLayoutPoller {
    type Message = CanvasMessage;

    fn receiver(&self) -> &std::sync::mpsc::Receiver<Self::Message> {
        &self.receiver
    }
}

struct Pixel {
    device_index: usize,
    color_new: Color,
    color_old: Color,
}

fn transform_color(color: Color, source: f32, target: f32) -> Color {
    // this is math :ghost:
    // and it doesn't work :ghost: nvm it does now
    (color - 1.0) * (1.0 - target) / (1.0 - source) + 1.0
}

/// Imagine this - you have multiple launchpads, you line them up, and now you use the Launchpads
/// as if they were a single device?! You can do that, with [`CanvasLayout`].
///
/// Create a layout, add [`Canvas`]es to it at the position where they appear on your table, and
/// you're ready to rock!
///
/// Example:
/// ```no_run
/// # use launchy::{CanvasLayout, Canvas as _};
/// let mut canvas_layout = CanvasLayout::new(|msg| println!("Got a message: {:?}", msg));
///
/// // Assuming you have a Launchpad MK2 and a Launchpad S lying next to it:
/// canvas_layout.add_by_guess::<launchy::mk2::Canvas>(0, 0)?;
/// canvas_layout.add_by_guess::<launchy::s::Canvas>(9, 0)?;
///
/// // Light the entire canvas layout red - i.e. both Launchpads will be red
/// for pad in canvas_layout.iter() {
///     canvas_layout[pad] = launchy::Color::RED;
/// }
/// canvas_layout.flush()?;
/// # Ok::<(), launchy::MidiError>(())
/// ```
pub struct CanvasLayout<'a> {
    devices: Vec<LayoutDevice<'a>>,
    coordinate_map: HashMap<(u32, u32), Pixel>, // we need to store some stuff for each pixel
    callback: std::sync::Arc<dyn Fn(CanvasMessage) + Send + Sync + 'static>,
    light_threshold: f32,
}

impl<'a> CanvasLayout<'a> {
    /// Create a new CanvasLayout that sends messages to the provided callback. The callback must
    /// implement `Fn` because it may be called from multiple devices concurrently.
    pub fn new(callback: impl Fn(CanvasMessage) + Send + Sync + 'static) -> Self {
        Self {
            devices: Vec::new(),
            coordinate_map: HashMap::new(),
            callback: std::sync::Arc::new(callback),
            light_threshold: 1.0 / 4.0, // good default value? I have, like, no idea
        }
    }

    /// Create a new CanvasLayout, plus an input handler object that you can use to poll messages.
    pub fn new_polling() -> (Self, CanvasLayoutPoller) {
        let (sender, receiver) = std::sync::mpsc::sync_channel(50);
        let canvas = Self::new(move |msg| {
            sender
                .send(msg)
                .expect("Message receiver has hung up (this shouldn't happen)")
        });

        let poller = CanvasLayoutPoller { receiver };

        (canvas, poller)
    }

    pub fn light_threshold(&self) -> f32 {
        self.light_threshold
    }
    pub fn set_light_threshold(&mut self, value: f32) {
        self.light_threshold = value
    }

    /// Add a new device to this canvas layout, at the specified `x` and `y` coordinate.
    ///
    /// The usage of this method is a bit awkward out of necessity. You need to provide a closure
    /// which, when called with a message callback, is expected to return a [`Canvas`] that is set
    /// up to deliver messsages to the provided message callback.
    ///
    /// The `Result` which the closure returns will be propagated.
    ///
    /// Example:
    /// ```no_run
    /// # use launchy::{CanvasLayout, Rotation};
    /// # let mut canvas_layout = launchy::CanvasLayout::new(|_| {});
    /// canvas_layout.add(0, 0, Rotation::None, |callback| launchy::mk2::Canvas::guess(callback))?;
    ///
    /// // or even nested layouts:
    /// canvas_layout.add(0, 0, Rotation::None, |callback| {
    ///     let mut canvas_layout = CanvasLayout::new(callback);
    ///     canvas_layout.add(0, 0, Rotation::None, |callback| launchy::mk2::Canvas::guess(callback))?;
    ///     Ok::<_, launchy::MidiError>(canvas_layout)
    /// })?;
    ///
    /// # Ok::<(), launchy::MidiError>(())
    /// ```
    ///
    /// If you want an easier way to add simple devices, see `add_by_guess`.
    pub fn add<C: 'a + Canvas, F, E>(
        &mut self,
        x_offset: u32,
        y_offset: u32,
        rotation: Rotation,
        creator: F,
    ) -> Result<(), E>
    where
        F: FnOnce(Box<dyn Fn(CanvasMessage) + Send + Sync + 'static>) -> Result<C, E>,
    {
        let callback = self.callback.clone();
        let canvas = (creator)(Box::new(move |msg| {
            let (x, y) = to_global(msg.x(), msg.y(), rotation, x_offset, y_offset);
            match msg {
                CanvasMessage::Press { .. } => (callback)(CanvasMessage::Press { x, y }),
                CanvasMessage::Release { .. } => (callback)(CanvasMessage::Release { x, y }),
            }
        }))?;

        let index = self.devices.len(); // The index of soon-to-be inserted object

        for pad in canvas.iter() {
            let translated_coords =
                to_global(pad.x as u32, pad.y as u32, rotation, x_offset, y_offset);
            let old_value = self.coordinate_map.insert(
                translated_coords,
                Pixel {
                    device_index: index,
                    color_new: canvas.get_pending(pad).unwrap(),
                    color_old: canvas[pad],
                },
            );

            // check for overlap
            if let Some(Pixel {
                device_index: old_device_index,
                ..
            }) = old_value
            {
                panic!(
                    "Found overlap at ({}|{})! with canvas {} while adding canvas {} to layout (zero-indexed)",
                    translated_coords.0, translated_coords.1, old_device_index, self.devices.len(),
                );
            }
        }

        let layout_device = LayoutDevice {
            canvas: Box::new(canvas),
            rotation,
            x: x_offset,
            y: y_offset,
        };
        self.devices.push(layout_device);

        Ok(())
    }

    /// Add a new device to this canvas, at the specified `x` and `y` coordinates. The MIDI
    /// connections used for communication with the underlying hardware are determined by guessing
    /// based on the device name.
    ///
    /// Specifiy the type of device using a generic Canvas type parameter.
    ///
    /// Example
    /// ```no_run
    /// # let mut canvas_layout = launchy::CanvasLayout::new(|_| {});
    /// // Assuming a Launchpad MK2 and a Launchpad S next to it:
    /// canvas_layout.add_by_guess::<launchy::mk2::Canvas>(0, 0)?;
    /// canvas_layout.add_by_guess::<launchy::s::Canvas>(9, 0)?;
    /// # Ok::<(), launchy::MidiError>(())
    /// ```
    pub fn add_by_guess<E: 'a + DeviceCanvasTrait>(
        &mut self,
        x: u32,
        y: u32,
    ) -> Result<(), crate::MidiError> {
        self.add(x, y, Rotation::None, DeviceCanvas::<E::Spec>::guess)
    }

    /// Like `add_by_guess`, but with a parameter for the rotation of the Launchpad.
    pub fn add_by_guess_rotated<E: 'a + DeviceCanvasTrait>(
        &mut self,
        x: u32,
        y: u32,
        rotation: Rotation,
    ) -> Result<(), crate::MidiError> {
        self.add(x, y, rotation, DeviceCanvas::<E::Spec>::guess)
    }
}

impl Canvas for CanvasLayout<'_> {
    fn lowest_visible_brightness(&self) -> f32 {
        self.light_threshold
    }

    fn bounding_box(&self) -> (u32, u32) {
        let mut width = 0;
        let mut height = 0;

        for device in &self.devices {
            let (device_width, device_height) = device.canvas.bounding_box();

            width = u32::max(width, device_width);
            height = u32::max(height, device_height);
        }

        (width, height)
    }

    fn low_level_get_pending(&self, x: u32, y: u32) -> Option<&Color> {
        let pixel = self.coordinate_map.get(&(x, y))?;
        Some(&pixel.color_new)
    }

    fn low_level_get_pending_mut(&mut self, x: u32, y: u32) -> Option<&mut Color> {
        // store the actual pixel color for possible retrieval later
        let pixel = self.coordinate_map.get_mut(&(x, y))?;
        Some(&mut pixel.color_new)
    }

    fn low_level_get(&self, x: u32, y: u32) -> Option<&Color> {
        let pixel = self.coordinate_map.get(&(x, y))?;
        Some(&pixel.color_old)
    }

    fn flush(&mut self) -> Result<(), crate::MidiError> {
        for (&(global_x, global_y), pixel) in self.coordinate_map.iter_mut() {
            let device = &mut self.devices[pixel.device_index];

            let transformed_color = transform_color(
                pixel.color_new,
                self.light_threshold,
                device.canvas.lowest_visible_brightness(),
            );

            let (local_x, local_y) = device.to_local(global_x, global_y);

            *device
                .canvas
                .low_level_get_pending_mut(local_x, local_y)
                .unwrap() = transformed_color;

            pixel.color_old = pixel.color_new;
        }

        for device in &mut self.devices {
            device.canvas.flush()?;
        }

        Ok(())
    }
}

impl_traits_for_canvas!(CanvasLayout['a]);
