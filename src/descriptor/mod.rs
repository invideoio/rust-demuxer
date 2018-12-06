//! Descriptors provide metadata about an element of a Transport Stream.
//!
//! For example, a descriptor may be used to specify the language of an audio track.  Use of
//! specific descriptors is often not mandatory (many streams do not describe the language of their
//! audio).
//!
//! The syntax
//! of specific PSI tables often allows descriptors to be attached to the table itself, or to
//! entries within the table.
//!
//! # Extensions
//!
//! Descriptors are a point of extension, with a range of descriptor types defined by the core
//! standard, and further descriptor types defined by standards based upon transport streams.  In
//! order to support this extension, while avoiding allocations (a la `dyn Trait`),
//! descriptor-related types and methods within this crate have a type-parameter so that calling
//! code which wants to use externally-defined descriptors can supply a type which supports them.
//!
//! So for example code using [`PmtSection`](..//demultiplex/struct.PmtSection.html) will need to
//! specify the `Descriptor` implementation to be produced,
//!
//! ```
//! # use mpeg2ts_reader::psi::pmt::PmtSection;
//! # use mpeg2ts_reader::descriptor::CoreDescriptors;
//! # let data = [0; 4];
//! let pmt = PmtSection::from_bytes(&data).unwrap();
//! // type parameter to descriptors() is inferred from the use of CoreDescriptors below
//! for d in pmt.descriptors() {
//!     if let Ok(CoreDescriptors::Registration(reg)) = d {
//!         println!("registration_descriptor {:#x}", reg.format_identifier());
//!     }
//! }
//! ```

pub mod iso_639_language;
pub mod registration;

use self::iso_639_language::Iso639LanguageDescriptor;
use self::registration::RegistrationDescriptor;
use std::fmt;
use std::marker;

/// Trait allowing users of this trait to supply their own implementation of descriptor parsing.
///
/// The default implementation provided by this crate is
/// [`CoreDescriptors`](enum.CoreDescriptors.html), which will only provide support for descriptor
/// types directly defined by _ISO/IEC 13818-1_.  To support descriptors from other standards,
/// an alternative implementation of this trait may be passed as a type parameter to methods such as
/// [`PmtSection::descriptors()`](..//demultiplex/struct.PmtSection.html#method.descriptors).
///
/// The [`descriptor_enum!{}`](../macro.descriptor_enum.html) macro can be used to help create
/// implementations of this trait.
pub trait Descriptor<'buf>: Sized {
    /// Create an object that that can wrap and parse the type of descriptor at the start of the
    /// given slice.
    fn from_bytes(buf: &'buf [u8]) -> Result<Self, DescriptorError>;
}

#[macro_export]
macro_rules! descriptor_enum {
    (
        $(#[$outer:meta])*
        $name:ident {
            $(
                $(#[$inner:ident $($args:tt)*])*
                $case_name:ident $($tags:pat)|* => $t:ident
            ),*,
        }
    ) => {
        $(#[$outer])*
        pub enum $name<'buf> {
            $(
                $(#[$inner $($args)*])*
                $case_name($t<'buf>),
            )*
        }
        impl<'buf> $crate::descriptor::Descriptor<'buf> for $name<'buf> {
            fn from_bytes(buf: &'buf[u8]) -> Result<Self, $crate::descriptor::DescriptorError> {
                if buf.len() <  2 {
                    return Err($crate::descriptor::DescriptorError::BufferTooShort{ buflen: buf.len() })
                }
                let tag = buf[0];
                let len = buf[1] as usize;
                let tag_end = len + 2;
                if tag_end > buf.len() {
                    return Err($crate::descriptor::DescriptorError::TagTooLongForBuffer{ taglen: len, buflen: buf.len() })
                }
                let payload = &buf[2..tag_end];
                match tag {
                    $( $( $tags )|* => Ok($name::$case_name($t::new(tag, payload)?)), )*
                    _ => Err($crate::descriptor::DescriptorError::UnhandledTagValue(tag)),
                }
            }
        }
    }
}

/// Catch-all type for when there is no explicit handling for the given descriptor type.
pub struct UnknownDescriptor<'buf> {
    /// the descriptor's identifying 'tag' value; different types of descriptors are assigned
    /// different tag values
    pub tag: u8,
    /// the descriptor's payload bytes
    pub payload: &'buf [u8],
}
impl<'buf> UnknownDescriptor<'buf> {
    /// Constructor, in the form required for use with the
    /// [`descriptor_enum!{}`](../macro.descriptor_enum.html) macro.
    pub fn new(tag: u8, payload: &'buf [u8]) -> Result<UnknownDescriptor<'buf>, DescriptorError> {
        Ok(UnknownDescriptor { tag, payload })
    }
}
impl<'buf> fmt::Debug for UnknownDescriptor<'buf> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("UnknownDescriptor")
            .field("tag", &self.tag)
            .field("len", &self.payload.len())
            .finish()
    }
}

descriptor_enum!{
    /// Default implementation of [`Descriptor`](trait.Descriptor.html) covering descriptor types
    /// from _ISO/IEC 13818-1_.
    ///
    /// **NB** coverage of the range of descriptors from the spec with descriptor-type-specific
    /// Rust types is currently incomplete, with those variants currently containing
    /// `UnknownDescriptor` needing to be changed to have type-specific implementations in some
    /// future release of this crate.
    #[derive(Debug)]
    CoreDescriptors {
        /// descriptor tag values `0`, `1` and `36` to `63` inclusive are marked as reserved by _ISO/IEC 13818-1_.
        Reserved 0|1|36..=63 => UnknownDescriptor,
        /// The `video_stream_descriptor()` syntax element from _ISO/IEC 13818-1_.
        VideoStream 2 => UnknownDescriptor,
        /// The `audio_stream_descriptor()` syntax element from _ISO/IEC 13818-1_.
        AudioStream 3 => UnknownDescriptor,
        /// The `hierarchy_descriptor()` syntax element from _ISO/IEC 13818-1_.
        Hierarchy 4 => UnknownDescriptor,
        /// The `registration_descriptor()` syntax element from _ISO/IEC 13818-1_.
        Registration 5 => RegistrationDescriptor,
        /// The `data_stream_alignment_descriptor()` syntax element from _ISO/IEC 13818-1_.
        DataStreamAlignment 6 => UnknownDescriptor,
        /// The `target_background_grid_descriptor()` syntax element from _ISO/IEC 13818-1_.
        TargetBackgroundGrid 7 => UnknownDescriptor,
        /// The `video_window_descriptor()` syntax element from _ISO/IEC 13818-1_.
        VideoWindow 8 => UnknownDescriptor,
        /// The `CA_descriptor()` syntax element from _ISO/IEC 13818-1_ ("Conditional Access").
        CA 9 => UnknownDescriptor,
        /// The `ISO_639_language_descriptor()` syntax element from _ISO/IEC 13818-1_.
        ISO639Language 10 => Iso639LanguageDescriptor,
        /// The `system_clock_descriptor()` syntax element from _ISO/IEC 13818-1_.
        SystemClock 11 => UnknownDescriptor,
        /// The `multiplex_buffer_utilization_descriptor()` syntax element from _ISO/IEC 13818-1_.
        MultiplexBufferUtilization 12 => UnknownDescriptor,
        /// The `copyright_descriptor()` syntax element from _ISO/IEC 13818-1_.
        Copyright 13 => UnknownDescriptor,
        /// The `maximum_bitrate_descriptor()` syntax element from _ISO/IEC 13818-1_.
        MaximumBitrate 14 => UnknownDescriptor,
        /// The `private_data_indicator_descriptor()` syntax element from _ISO/IEC 13818-1_.
        PrivateDataIndicator 15 => UnknownDescriptor,
        /// The `smoothing_buffer_descriptor()` syntax element from _ISO/IEC 13818-1_.
        SmoothingBuffer 16 => UnknownDescriptor,
        /// The `STD_descriptor()` syntax element from _ISO/IEC 13818-1_.
        STD 17 => UnknownDescriptor,
        /// The `ibp_descriptor()` syntax element from _ISO/IEC 13818-1_.
        IBP 18 => UnknownDescriptor,
        /// descriptor tag values `19` to `26` inclusive are marked as reserved by _ISO IEC 13818-6_ (NB a different standard than the one supported by this crate).
        IsoIec13818dash6 19..=26 => UnknownDescriptor,
        /// The `MPEG-4_video_descriptor()` syntax element from _ISO/IEC 13818-1_.
        MPEG4Video 27 => UnknownDescriptor,
        /// The `MPEG-4_audio_descriptor()` syntax element from _ISO/IEC 13818-1_.
        MPEG4Audio 28 => UnknownDescriptor,
        /// The `IOD_descriptor()` syntax element from _ISO/IEC 13818-1_ ("Initial Object Descriptor").
        IOD 29 => UnknownDescriptor,
        /// The `SL_descriptor()` syntax element from _ISO/IEC 13818-1_ ("Synchronization Layer").
        SL 30 => UnknownDescriptor,
        /// The `FMC_descriptor()` syntax element from _ISO/IEC 13818-1_ ("FlexMux Channel").
        FMC 31 => UnknownDescriptor,
        /// The `External_ES_ID_descriptor()` syntax element from _ISO/IEC 13818-1_.
        ExternalESID 32 => UnknownDescriptor,
        /// The `Muxcode_descriptor()` syntax element from _ISO/IEC 13818-1_.
        MuxCode 33 => UnknownDescriptor,
        /// The `FmxBufferSize_descriptor()` syntax element from _ISO/IEC 13818-1_ ("FlexMux buffer").
        FmxBufferSize 34 => UnknownDescriptor,
        /// The `MultiplexBuffer_descriptor()` syntax element from _ISO/IEC 13818-1_.
        MultiplexBuffer 35 => UnknownDescriptor,
        /// descriptor tag values `64` to `255` inclusive are marked for 'use private' use by _ISO/IEC 13818-1_.
        UserPrivate 64..=255 => UnknownDescriptor,
    }
}

/// Iterator over the descriptor elements in a given byte slice.
pub struct DescriptorIter<'buf, Desc>
where
    Desc: Descriptor<'buf>,
{
    buf: &'buf [u8],
    phantom: marker::PhantomData<Desc>,
}
impl<'buf, Desc> DescriptorIter<'buf, Desc>
where
    Desc: Descriptor<'buf>,
{
    /// Create an iterator over all the descriptors in the given slice
    pub fn new(buf: &'buf [u8]) -> DescriptorIter<'buf, Desc> {
        DescriptorIter {
            buf,
            phantom: marker::PhantomData,
        }
    }
}
impl<'buf, Desc> Iterator for DescriptorIter<'buf, Desc>
where
    Desc: Descriptor<'buf>,
{
    type Item = Result<Desc, DescriptorError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.is_empty() {
            return None;
        }
        let tag = self.buf[0];
        let len = self.buf[1] as usize;
        let remaining_size = self.buf.len() - 2;
        if len > remaining_size {
            // ensure anther call to next() will yield None,
            self.buf = &self.buf[0..0];
            Some(Err(DescriptorError::NotEnoughData {
                tag,
                actual: remaining_size,
                expected: len,
            }))
        } else {
            let (desc, rest) = self.buf.split_at(len + 2);
            self.buf = rest;
            Some(Descriptor::from_bytes(desc))
        }
    }
}

/// An error during parsing of a descriptor
#[derive(Debug, PartialEq)]
pub enum DescriptorError {
    /// The amount of data available in the buffer is not enough to hold the descriptor's declared
    /// size.
    NotEnoughData {
        /// descriptor tag value
        tag: u8,
        /// actual buffer size
        actual: usize,
        /// expected buffer size
        expected: usize,
    },
    /// TODO: replace with NotEnoughData
    TagTooLongForBuffer {
        /// actual length in descriptor header
        taglen: usize,
        /// remaining bytes in buffer (which is seen to be shorter than `taglen`)
        buflen: usize,
    },
    /// The buffer is too short to even hold the two bytes of generic descriptor header data
    BufferTooShort {
        /// the actual buffer length
        buflen: usize,
    },
    /// There is no mapping defined of the given descriptor tag value to a `Descriptor` value.
    UnhandledTagValue(u8),
}
