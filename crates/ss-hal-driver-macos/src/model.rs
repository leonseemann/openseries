//! The driver's object tree and its properties as plain Rust values
//! (`docs/architecture/virtual-mic-driver.md` §2). The FFI layer serialises these values into
//! the C types the HAL expects.

use crate::selectors as sel;

/// Sample rate of both devices (fixed).
pub const SAMPLE_RATE: f64 = 48_000.0;
/// Channels of both streams.
pub const CHANNELS: u32 = 1;
/// Frames per zero time stamp and ring capacity.
pub const PERIOD_FRAMES: u32 = 16_384;
/// Bytes per sample (`f32`).
const BYTES_PER_SAMPLE: u32 = 4;

/// `kAudioObjectPlugInObject`.
pub const PLUGIN_ID: u32 = 1;
/// The visible microphone device.
pub const MIC_DEVICE_ID: u32 = 2;
/// Input stream of the microphone device.
pub const MIC_STREAM_ID: u32 = 3;
/// The hidden feed device the daemon writes into.
pub const FEED_DEVICE_ID: u32 = 4;
/// Output stream of the feed device.
pub const FEED_STREAM_ID: u32 = 5;
/// `kAudioObjectUnknown`.
const NO_OBJECT: u32 = 0;

/// Manufacturer shown for the plug-in and devices.
pub const MANUFACTURER: &str = "StealSeries";
/// Clock domain shared by both devices (any non-zero value unique to this driver).
const CLOCK_DOMAIN: u32 = sel::fourcc(b"stsm");

const DEVICE_LIST: &[u32] = &[MIC_DEVICE_ID, FEED_DEVICE_ID];
const MIC_STREAMS: &[u32] = &[MIC_STREAM_ID];
const FEED_STREAMS: &[u32] = &[FEED_STREAM_ID];
const MIC_RELATED: &[u32] = &[MIC_DEVICE_ID];
const FEED_RELATED: &[u32] = &[FEED_DEVICE_ID];
const NONE: &[u32] = &[];
const SAMPLE_RATES: &[(f64, f64)] = &[(SAMPLE_RATE, SAMPLE_RATE)];

/// One of the two devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// "StealSeries Microphone": visible, input only; apps read from it.
    Mic,
    /// "StealSeries Microphone Feed": hidden, output only; the daemon writes into it.
    Feed,
}

impl DeviceKind {
    /// Both devices.
    pub const ALL: [Self; 2] = [Self::Mic, Self::Feed];

    /// Object ID of the device.
    #[must_use]
    pub const fn device_id(self) -> u32 {
        match self {
            Self::Mic => MIC_DEVICE_ID,
            Self::Feed => FEED_DEVICE_ID,
        }
    }

    /// Persistent UID (what the daemon and saved app settings refer to).
    #[must_use]
    pub const fn uid(self) -> &'static str {
        match self {
            Self::Mic => "dev.stealseries.mic",
            Self::Feed => "dev.stealseries.mic.feed",
        }
    }

    /// Display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Mic => "StealSeries Microphone",
            Self::Feed => "StealSeries Microphone Feed",
        }
    }

    /// The scope the device's only stream lives in.
    const fn stream_scope(self) -> u32 {
        match self {
            Self::Mic => sel::SCOPE_INPUT,
            Self::Feed => sel::SCOPE_OUTPUT,
        }
    }

    const fn streams(self) -> &'static [u32] {
        match self {
            Self::Mic => MIC_STREAMS,
            Self::Feed => FEED_STREAMS,
        }
    }

    /// The IO operation this device performs on its ring side.
    #[must_use]
    pub const fn io_operation(self) -> u32 {
        match self {
            Self::Mic => sel::IO_READ_INPUT,
            Self::Feed => sel::IO_WRITE_MIX,
        }
    }
}

/// An object published by the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Object {
    /// The plug-in itself.
    PlugIn,
    /// A device.
    Device(DeviceKind),
    /// The single stream of a device.
    Stream(DeviceKind),
}

impl Object {
    /// Resolves an `AudioObjectID`.
    #[must_use]
    pub const fn from_id(id: u32) -> Option<Self> {
        match id {
            PLUGIN_ID => Some(Self::PlugIn),
            MIC_DEVICE_ID => Some(Self::Device(DeviceKind::Mic)),
            MIC_STREAM_ID => Some(Self::Stream(DeviceKind::Mic)),
            FEED_DEVICE_ID => Some(Self::Device(DeviceKind::Feed)),
            FEED_STREAM_ID => Some(Self::Stream(DeviceKind::Feed)),
            _ => None,
        }
    }
}

/// The fixed stream format: 48 kHz mono packed `f32`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamFormat {
    /// Frames per second.
    pub sample_rate: f64,
    /// `kAudioFormatLinearPCM`.
    pub format_id: u32,
    /// Float | packed.
    pub format_flags: u32,
    /// Bytes per packet (= per frame).
    pub bytes_per_packet: u32,
    /// Always 1 for PCM.
    pub frames_per_packet: u32,
    /// Bytes per frame.
    pub bytes_per_frame: u32,
    /// Channels per frame.
    pub channels_per_frame: u32,
    /// Bits per channel.
    pub bits_per_channel: u32,
}

/// The only format both streams support.
pub const FORMAT: StreamFormat = StreamFormat {
    sample_rate: SAMPLE_RATE,
    format_id: sel::FORMAT_LINEAR_PCM,
    format_flags: sel::FORMAT_FLAGS_FLOAT_PACKED,
    bytes_per_packet: BYTES_PER_SAMPLE * CHANNELS,
    frames_per_packet: 1,
    bytes_per_frame: BYTES_PER_SAMPLE * CHANNELS,
    channels_per_frame: CHANNELS,
    bits_per_channel: BYTES_PER_SAMPLE * 8,
};

/// A property value before serialisation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PropertyValue {
    /// `UInt32` (also IDs, booleans and four-char codes).
    U32(u32),
    /// `Float64`.
    F64(f64),
    /// Array of `AudioObjectID`.
    ObjectIds(&'static [u32]),
    /// `CFStringRef` (caller releases).
    String(&'static str),
    /// One `AudioStreamBasicDescription`.
    Format(StreamFormat),
    /// One `AudioStreamRangedDescription` (format plus its sample-rate range).
    FormatList(StreamFormat),
    /// Array of `AudioValueRange`.
    Ranges(&'static [(f64, f64)]),
    /// `kAudioPlugInPropertyTranslateUIDToDevice`: needs the `CFString` qualifier; see
    /// [`device_for_uid`].
    TranslateUid,
}

/// Mutable state that some properties report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeState {
    /// Whether IO is running on the microphone device.
    pub mic_running: bool,
    /// Whether IO is running on the feed device.
    pub feed_running: bool,
}

impl RuntimeState {
    const fn is_running(self, kind: DeviceKind) -> bool {
        match kind {
            DeviceKind::Mic => self.mic_running,
            DeviceKind::Feed => self.feed_running,
        }
    }
}

/// The value of `selector` in `scope` on `object`, or `None` if the object has no such property.
#[must_use]
pub fn property(
    object: Object,
    selector: u32,
    scope: u32,
    state: RuntimeState,
) -> Option<PropertyValue> {
    match object {
        Object::PlugIn => plugin_property(selector),
        Object::Device(kind) => device_property(kind, selector, scope, state),
        Object::Stream(kind) => stream_property(kind, selector),
    }
}

/// The device with the given UID.
#[must_use]
pub fn device_for_uid(uid: &str) -> Option<u32> {
    DeviceKind::ALL
        .into_iter()
        .find(|kind| kind.uid() == uid)
        .map(DeviceKind::device_id)
}

fn plugin_property(selector: u32) -> Option<PropertyValue> {
    use PropertyValue as V;
    Some(match selector {
        sel::BASE_CLASS => V::U32(sel::CLASS_OBJECT),
        sel::CLASS => V::U32(sel::CLASS_PLUGIN),
        sel::OWNER => V::U32(NO_OBJECT),
        sel::MANUFACTURER => V::String(MANUFACTURER),
        sel::OWNED_OBJECTS | sel::PLUGIN_DEVICE_LIST => V::ObjectIds(DEVICE_LIST),
        sel::PLUGIN_TRANSLATE_UID_TO_DEVICE => V::TranslateUid,
        sel::PLUGIN_RESOURCE_BUNDLE => V::String(""),
        _ => return None,
    })
}

fn device_property(
    kind: DeviceKind,
    selector: u32,
    scope: u32,
    state: RuntimeState,
) -> Option<PropertyValue> {
    use PropertyValue as V;
    let in_scope =
        scope == kind.stream_scope() || scope == sel::SCOPE_GLOBAL || scope == sel::SCOPE_WILDCARD;
    let streams = if in_scope { kind.streams() } else { NONE };
    Some(match selector {
        sel::BASE_CLASS => V::U32(sel::CLASS_OBJECT),
        sel::CLASS => V::U32(sel::CLASS_DEVICE),
        sel::OWNER => V::U32(PLUGIN_ID),
        sel::NAME => V::String(kind.name()),
        sel::MANUFACTURER => V::String(MANUFACTURER),
        sel::OWNED_OBJECTS | sel::DEVICE_STREAMS => V::ObjectIds(streams),
        sel::DEVICE_UID => V::String(kind.uid()),
        sel::DEVICE_MODEL_UID => V::String("dev.stealseries.mic.model"),
        sel::DEVICE_TRANSPORT_TYPE => V::U32(sel::TRANSPORT_VIRTUAL),
        sel::DEVICE_RELATED_DEVICES => V::ObjectIds(match kind {
            DeviceKind::Mic => MIC_RELATED,
            DeviceKind::Feed => FEED_RELATED,
        }),
        sel::DEVICE_CLOCK_DOMAIN => V::U32(CLOCK_DOMAIN),
        sel::DEVICE_IS_ALIVE | sel::DEVICE_CLOCK_IS_STABLE => V::U32(1),
        sel::DEVICE_IS_RUNNING => V::U32(u32::from(state.is_running(kind))),
        sel::DEVICE_CAN_BE_DEFAULT => V::U32(u32::from(
            kind == DeviceKind::Mic && scope == sel::SCOPE_INPUT,
        )),
        sel::DEVICE_CAN_BE_DEFAULT_SYSTEM | sel::LATENCY | sel::DEVICE_SAFETY_OFFSET => V::U32(0),
        sel::CONTROL_LIST => V::ObjectIds(NONE),
        sel::DEVICE_NOMINAL_SAMPLE_RATE => V::F64(SAMPLE_RATE),
        sel::DEVICE_AVAILABLE_SAMPLE_RATES => V::Ranges(SAMPLE_RATES),
        sel::DEVICE_IS_HIDDEN => V::U32(u32::from(kind == DeviceKind::Feed)),
        sel::DEVICE_ZERO_TIMESTAMP_PERIOD => V::U32(PERIOD_FRAMES),
        _ => return None,
    })
}

fn stream_property(kind: DeviceKind, selector: u32) -> Option<PropertyValue> {
    use PropertyValue as V;
    Some(match selector {
        sel::BASE_CLASS => V::U32(sel::CLASS_OBJECT),
        sel::CLASS => V::U32(sel::CLASS_STREAM),
        sel::OWNER => V::U32(kind.device_id()),
        sel::OWNED_OBJECTS => V::ObjectIds(NONE),
        sel::STREAM_IS_ACTIVE | sel::STREAM_STARTING_CHANNEL => V::U32(1),
        sel::STREAM_DIRECTION => V::U32(u32::from(kind == DeviceKind::Mic)),
        sel::STREAM_TERMINAL_TYPE => V::U32(match kind {
            DeviceKind::Mic => sel::TERMINAL_MICROPHONE,
            DeviceKind::Feed => sel::TERMINAL_LINE,
        }),
        sel::LATENCY => V::U32(0),
        sel::STREAM_VIRTUAL_FORMAT | sel::STREAM_PHYSICAL_FORMAT => V::Format(FORMAT),
        sel::STREAM_AVAILABLE_VIRTUAL_FORMATS | sel::STREAM_AVAILABLE_PHYSICAL_FORMATS => {
            V::FormatList(FORMAT)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(id: u32, selector: u32, scope: u32) -> Option<PropertyValue> {
        property(
            Object::from_id(id)?,
            selector,
            scope,
            RuntimeState::default(),
        )
    }

    #[test]
    fn plugin_lists_both_devices() {
        assert_eq!(
            get(PLUGIN_ID, sel::PLUGIN_DEVICE_LIST, sel::SCOPE_GLOBAL),
            Some(PropertyValue::ObjectIds(&[MIC_DEVICE_ID, FEED_DEVICE_ID]))
        );
    }

    #[test]
    fn streams_are_filtered_by_scope() {
        let input = PropertyValue::ObjectIds(&[MIC_STREAM_ID]);
        assert_eq!(
            get(MIC_DEVICE_ID, sel::DEVICE_STREAMS, sel::SCOPE_INPUT),
            Some(input)
        );
        assert_eq!(
            get(MIC_DEVICE_ID, sel::DEVICE_STREAMS, sel::SCOPE_OUTPUT),
            Some(PropertyValue::ObjectIds(&[]))
        );
        assert_eq!(
            get(FEED_DEVICE_ID, sel::DEVICE_STREAMS, sel::SCOPE_OUTPUT),
            Some(PropertyValue::ObjectIds(&[FEED_STREAM_ID]))
        );
    }

    #[test]
    fn only_the_feed_is_hidden_and_only_the_mic_can_be_default_input() {
        assert_eq!(
            get(FEED_DEVICE_ID, sel::DEVICE_IS_HIDDEN, sel::SCOPE_GLOBAL),
            Some(PropertyValue::U32(1))
        );
        assert_eq!(
            get(MIC_DEVICE_ID, sel::DEVICE_IS_HIDDEN, sel::SCOPE_GLOBAL),
            Some(PropertyValue::U32(0))
        );
        assert_eq!(
            get(MIC_DEVICE_ID, sel::DEVICE_CAN_BE_DEFAULT, sel::SCOPE_INPUT),
            Some(PropertyValue::U32(1))
        );
        assert_eq!(
            get(
                FEED_DEVICE_ID,
                sel::DEVICE_CAN_BE_DEFAULT,
                sel::SCOPE_OUTPUT
            ),
            Some(PropertyValue::U32(0))
        );
    }

    #[test]
    fn stream_directions_and_format() {
        assert_eq!(
            get(MIC_STREAM_ID, sel::STREAM_DIRECTION, sel::SCOPE_GLOBAL),
            Some(PropertyValue::U32(1))
        );
        assert_eq!(
            get(FEED_STREAM_ID, sel::STREAM_DIRECTION, sel::SCOPE_GLOBAL),
            Some(PropertyValue::U32(0))
        );
        assert_eq!(FORMAT.bytes_per_frame, 4);
        assert_eq!(FORMAT.bits_per_channel, 32);
    }

    #[test]
    fn running_state_is_reported_per_device() {
        let state = RuntimeState {
            mic_running: false,
            feed_running: true,
        };
        let running = |kind| {
            property(
                Object::Device(kind),
                sel::DEVICE_IS_RUNNING,
                sel::SCOPE_GLOBAL,
                state,
            )
        };
        assert_eq!(running(DeviceKind::Mic), Some(PropertyValue::U32(0)));
        assert_eq!(running(DeviceKind::Feed), Some(PropertyValue::U32(1)));
    }

    #[test]
    fn uid_lookup_and_unknown_objects() {
        assert_eq!(device_for_uid("dev.stealseries.mic"), Some(MIC_DEVICE_ID));
        assert_eq!(
            device_for_uid("dev.stealseries.mic.feed"),
            Some(FEED_DEVICE_ID)
        );
        assert_eq!(device_for_uid("other"), None);
        assert_eq!(Object::from_id(99), None);
        assert_eq!(
            get(MIC_DEVICE_ID, sel::fourcc(b"xxxx"), sel::SCOPE_GLOBAL),
            None
        );
    }
}
