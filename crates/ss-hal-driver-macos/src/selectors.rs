//! Four-character codes of the HAL plug-in interface, named after the constants in Apple's public
//! SDK headers `AudioHardwareBase.h`, `AudioHardware.h` and `AudioServerPlugIn.h`.
//!
//! Defined here (instead of taken from `objc2-core-audio`) so the property model compiles and is
//! tested on every OS; `tests::matches_sdk` cross-checks the values on macOS.

/// Builds a four-character code (`'abcd'` in C).
#[must_use]
pub const fn fourcc(code: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*code)
}

// Classes.
/// `kAudioObjectClassID`
pub const CLASS_OBJECT: u32 = fourcc(b"aobj");
/// `kAudioPlugInClassID`
pub const CLASS_PLUGIN: u32 = fourcc(b"aplg");
/// `kAudioDeviceClassID`
pub const CLASS_DEVICE: u32 = fourcc(b"adev");
/// `kAudioStreamClassID`
pub const CLASS_STREAM: u32 = fourcc(b"astr");

// Scopes and elements.
/// `kAudioObjectPropertyScopeGlobal`
pub const SCOPE_GLOBAL: u32 = fourcc(b"glob");
/// `kAudioObjectPropertyScopeInput`
pub const SCOPE_INPUT: u32 = fourcc(b"inpt");
/// `kAudioObjectPropertyScopeOutput`
pub const SCOPE_OUTPUT: u32 = fourcc(b"outp");
/// `kAudioObjectPropertyScopeWildcard`
pub const SCOPE_WILDCARD: u32 = fourcc(b"****");

// AudioObject properties.
/// `kAudioObjectPropertyBaseClass`
pub const BASE_CLASS: u32 = fourcc(b"bcls");
/// `kAudioObjectPropertyClass`
pub const CLASS: u32 = fourcc(b"clas");
/// `kAudioObjectPropertyOwner`
pub const OWNER: u32 = fourcc(b"stdv");
/// `kAudioObjectPropertyName`
pub const NAME: u32 = fourcc(b"lnam");
/// `kAudioObjectPropertyManufacturer`
pub const MANUFACTURER: u32 = fourcc(b"lmak");
/// `kAudioObjectPropertyOwnedObjects`
pub const OWNED_OBJECTS: u32 = fourcc(b"ownd");

// Plug-in properties.
/// `kAudioPlugInPropertyDeviceList`
pub const PLUGIN_DEVICE_LIST: u32 = fourcc(b"dev#");
/// `kAudioPlugInPropertyTranslateUIDToDevice`
pub const PLUGIN_TRANSLATE_UID_TO_DEVICE: u32 = fourcc(b"uidd");
/// `kAudioPlugInPropertyResourceBundle`
pub const PLUGIN_RESOURCE_BUNDLE: u32 = fourcc(b"rsrc");

// Device properties.
/// `kAudioDevicePropertyDeviceUID`
pub const DEVICE_UID: u32 = fourcc(b"uid ");
/// `kAudioDevicePropertyModelUID`
pub const DEVICE_MODEL_UID: u32 = fourcc(b"muid");
/// `kAudioDevicePropertyTransportType`
pub const DEVICE_TRANSPORT_TYPE: u32 = fourcc(b"tran");
/// `kAudioDevicePropertyRelatedDevices`
pub const DEVICE_RELATED_DEVICES: u32 = fourcc(b"akin");
/// `kAudioDevicePropertyClockDomain`
pub const DEVICE_CLOCK_DOMAIN: u32 = fourcc(b"clkd");
/// `kAudioDevicePropertyDeviceIsAlive`
pub const DEVICE_IS_ALIVE: u32 = fourcc(b"livn");
/// `kAudioDevicePropertyDeviceIsRunning`
pub const DEVICE_IS_RUNNING: u32 = fourcc(b"goin");
/// `kAudioDevicePropertyDeviceCanBeDefaultDevice`
pub const DEVICE_CAN_BE_DEFAULT: u32 = fourcc(b"dflt");
/// `kAudioDevicePropertyDeviceCanBeDefaultSystemDevice`
pub const DEVICE_CAN_BE_DEFAULT_SYSTEM: u32 = fourcc(b"sflt");
/// `kAudioDevicePropertyLatency` (also used on streams)
pub const LATENCY: u32 = fourcc(b"ltnc");
/// `kAudioDevicePropertyStreams`
pub const DEVICE_STREAMS: u32 = fourcc(b"stm#");
/// `kAudioObjectPropertyControlList`
pub const CONTROL_LIST: u32 = fourcc(b"ctrl");
/// `kAudioDevicePropertySafetyOffset`
pub const DEVICE_SAFETY_OFFSET: u32 = fourcc(b"saft");
/// `kAudioDevicePropertyNominalSampleRate`
pub const DEVICE_NOMINAL_SAMPLE_RATE: u32 = fourcc(b"nsrt");
/// `kAudioDevicePropertyAvailableNominalSampleRates`
pub const DEVICE_AVAILABLE_SAMPLE_RATES: u32 = fourcc(b"nsr#");
/// `kAudioDevicePropertyIsHidden`
pub const DEVICE_IS_HIDDEN: u32 = fourcc(b"hidn");
/// `kAudioDevicePropertyZeroTimeStampPeriod`
pub const DEVICE_ZERO_TIMESTAMP_PERIOD: u32 = fourcc(b"ring");
/// `kAudioDevicePropertyClockIsStable`
pub const DEVICE_CLOCK_IS_STABLE: u32 = fourcc(b"cstb");

// Stream properties.
/// `kAudioStreamPropertyIsActive`
pub const STREAM_IS_ACTIVE: u32 = fourcc(b"sact");
/// `kAudioStreamPropertyDirection`
pub const STREAM_DIRECTION: u32 = fourcc(b"sdir");
/// `kAudioStreamPropertyTerminalType`
pub const STREAM_TERMINAL_TYPE: u32 = fourcc(b"term");
/// `kAudioStreamPropertyStartingChannel`
pub const STREAM_STARTING_CHANNEL: u32 = fourcc(b"schn");
/// `kAudioStreamPropertyVirtualFormat`
pub const STREAM_VIRTUAL_FORMAT: u32 = fourcc(b"sfmt");
/// `kAudioStreamPropertyAvailableVirtualFormats`
pub const STREAM_AVAILABLE_VIRTUAL_FORMATS: u32 = fourcc(b"sfma");
/// `kAudioStreamPropertyPhysicalFormat`
pub const STREAM_PHYSICAL_FORMAT: u32 = fourcc(b"pft ");
/// `kAudioStreamPropertyAvailablePhysicalFormats`
pub const STREAM_AVAILABLE_PHYSICAL_FORMATS: u32 = fourcc(b"pfta");

// Property values.
/// `kAudioDeviceTransportTypeVirtual`
pub const TRANSPORT_VIRTUAL: u32 = fourcc(b"virt");
/// `kAudioStreamTerminalTypeMicrophone`
pub const TERMINAL_MICROPHONE: u32 = fourcc(b"micr");
/// `kAudioStreamTerminalTypeLine`
pub const TERMINAL_LINE: u32 = fourcc(b"line");
/// `kAudioFormatLinearPCM`
pub const FORMAT_LINEAR_PCM: u32 = fourcc(b"lpcm");
/// `kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked` (= `kAudioFormatFlagsNativeFloatPacked`
/// on little-endian Macs).
pub const FORMAT_FLAGS_FLOAT_PACKED: u32 = 0x1 | 0x8;

// IO operations (`AudioServerPlugInIOOperation`).
/// `kAudioServerPlugInIOOperationReadInput`
pub const IO_READ_INPUT: u32 = fourcc(b"read");
/// `kAudioServerPlugInIOOperationWriteMix`
pub const IO_WRITE_MIX: u32 = fourcc(b"rite");

// Status codes (`OSStatus`).
/// `kAudioHardwareNoError`
pub const STATUS_OK: i32 = 0;
/// `kAudioHardwareUnspecifiedError`
pub const STATUS_UNSPECIFIED: i32 = status(*b"what");
/// `kAudioHardwareUnknownPropertyError`
pub const STATUS_UNKNOWN_PROPERTY: i32 = status(*b"who?");
/// `kAudioHardwareBadPropertySizeError`
pub const STATUS_BAD_PROPERTY_SIZE: i32 = status(*b"!siz");
/// `kAudioHardwareIllegalOperationError`
pub const STATUS_ILLEGAL_OPERATION: i32 = status(*b"nope");
/// `kAudioHardwareBadObjectError`
pub const STATUS_BAD_OBJECT: i32 = status(*b"!obj");
/// `kAudioHardwareBadDeviceError`
pub const STATUS_BAD_DEVICE: i32 = status(*b"!dev");
/// `kAudioHardwareUnsupportedOperationError`
pub const STATUS_UNSUPPORTED_OPERATION: i32 = status(*b"unop");

const fn status(code: [u8; 4]) -> i32 {
    i32::from_be_bytes(code)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use objc2_core_audio as ca;

    #[test]
    #[allow(clippy::too_many_lines)] // one flat assertion table
    fn matches_sdk() {
        let pairs: &[(u32, u32)] = &[
            (CLASS_OBJECT, ca::kAudioObjectClassID),
            (CLASS_PLUGIN, ca::kAudioPlugInClassID),
            (CLASS_DEVICE, ca::kAudioDeviceClassID),
            (CLASS_STREAM, ca::kAudioStreamClassID),
            (SCOPE_GLOBAL, ca::kAudioObjectPropertyScopeGlobal),
            (SCOPE_INPUT, ca::kAudioObjectPropertyScopeInput),
            (SCOPE_OUTPUT, ca::kAudioObjectPropertyScopeOutput),
            (SCOPE_WILDCARD, ca::kAudioObjectPropertyScopeWildcard),
            (BASE_CLASS, ca::kAudioObjectPropertyBaseClass),
            (CLASS, ca::kAudioObjectPropertyClass),
            (OWNER, ca::kAudioObjectPropertyOwner),
            (NAME, ca::kAudioObjectPropertyName),
            (MANUFACTURER, ca::kAudioObjectPropertyManufacturer),
            (OWNED_OBJECTS, ca::kAudioObjectPropertyOwnedObjects),
            (PLUGIN_DEVICE_LIST, ca::kAudioPlugInPropertyDeviceList),
            (
                PLUGIN_TRANSLATE_UID_TO_DEVICE,
                ca::kAudioPlugInPropertyTranslateUIDToDevice,
            ),
            (
                PLUGIN_RESOURCE_BUNDLE,
                ca::kAudioPlugInPropertyResourceBundle,
            ),
            (DEVICE_UID, ca::kAudioDevicePropertyDeviceUID),
            (DEVICE_MODEL_UID, ca::kAudioDevicePropertyModelUID),
            (DEVICE_TRANSPORT_TYPE, ca::kAudioDevicePropertyTransportType),
            (
                DEVICE_RELATED_DEVICES,
                ca::kAudioDevicePropertyRelatedDevices,
            ),
            (DEVICE_CLOCK_DOMAIN, ca::kAudioDevicePropertyClockDomain),
            (DEVICE_IS_ALIVE, ca::kAudioDevicePropertyDeviceIsAlive),
            (DEVICE_IS_RUNNING, ca::kAudioDevicePropertyDeviceIsRunning),
            (
                DEVICE_CAN_BE_DEFAULT,
                ca::kAudioDevicePropertyDeviceCanBeDefaultDevice,
            ),
            (
                DEVICE_CAN_BE_DEFAULT_SYSTEM,
                ca::kAudioDevicePropertyDeviceCanBeDefaultSystemDevice,
            ),
            (LATENCY, ca::kAudioDevicePropertyLatency),
            (LATENCY, ca::kAudioStreamPropertyLatency),
            (DEVICE_STREAMS, ca::kAudioDevicePropertyStreams),
            (CONTROL_LIST, ca::kAudioObjectPropertyControlList),
            (DEVICE_SAFETY_OFFSET, ca::kAudioDevicePropertySafetyOffset),
            (
                DEVICE_NOMINAL_SAMPLE_RATE,
                ca::kAudioDevicePropertyNominalSampleRate,
            ),
            (
                DEVICE_AVAILABLE_SAMPLE_RATES,
                ca::kAudioDevicePropertyAvailableNominalSampleRates,
            ),
            (DEVICE_IS_HIDDEN, ca::kAudioDevicePropertyIsHidden),
            (
                DEVICE_ZERO_TIMESTAMP_PERIOD,
                ca::kAudioDevicePropertyZeroTimeStampPeriod,
            ),
            (
                DEVICE_CLOCK_IS_STABLE,
                ca::kAudioDevicePropertyClockIsStable,
            ),
            (STREAM_IS_ACTIVE, ca::kAudioStreamPropertyIsActive),
            (STREAM_DIRECTION, ca::kAudioStreamPropertyDirection),
            (STREAM_TERMINAL_TYPE, ca::kAudioStreamPropertyTerminalType),
            (
                STREAM_STARTING_CHANNEL,
                ca::kAudioStreamPropertyStartingChannel,
            ),
            (STREAM_VIRTUAL_FORMAT, ca::kAudioStreamPropertyVirtualFormat),
            (
                STREAM_AVAILABLE_VIRTUAL_FORMATS,
                ca::kAudioStreamPropertyAvailableVirtualFormats,
            ),
            (
                STREAM_PHYSICAL_FORMAT,
                ca::kAudioStreamPropertyPhysicalFormat,
            ),
            (
                STREAM_AVAILABLE_PHYSICAL_FORMATS,
                ca::kAudioStreamPropertyAvailablePhysicalFormats,
            ),
            (TRANSPORT_VIRTUAL, ca::kAudioDeviceTransportTypeVirtual),
            (TERMINAL_MICROPHONE, ca::kAudioStreamTerminalTypeMicrophone),
            (TERMINAL_LINE, ca::kAudioStreamTerminalTypeLine),
        ];
        for (index, (ours, sdk)) in pairs.iter().enumerate() {
            assert_eq!(ours, sdk, "selector #{index}");
        }
        assert_eq!(IO_READ_INPUT, ca::AudioServerPlugInIOOperation::ReadInput.0);
        assert_eq!(IO_WRITE_MIX, ca::AudioServerPlugInIOOperation::WriteMix.0);
        let statuses: &[(i32, i32)] = &[
            (STATUS_UNSPECIFIED, ca::kAudioHardwareUnspecifiedError),
            (
                STATUS_UNKNOWN_PROPERTY,
                ca::kAudioHardwareUnknownPropertyError,
            ),
            (
                STATUS_BAD_PROPERTY_SIZE,
                ca::kAudioHardwareBadPropertySizeError,
            ),
            (
                STATUS_ILLEGAL_OPERATION,
                ca::kAudioHardwareIllegalOperationError,
            ),
            (STATUS_BAD_OBJECT, ca::kAudioHardwareBadObjectError),
            (STATUS_BAD_DEVICE, ca::kAudioHardwareBadDeviceError),
            (
                STATUS_UNSUPPORTED_OPERATION,
                ca::kAudioHardwareUnsupportedOperationError,
            ),
        ];
        for (ours, sdk) in statuses {
            assert_eq!(ours, sdk);
        }
    }
}
