use crate::{
    EsmError, EsmReader, RefId,
    esm_record::EsmRecord,
    types::{enums::RecInt, flags::ObjectFlags},
};
use esm_parser_macros::LoadSave;

#[derive(Clone, Debug, Default, PartialEq, LoadSave)]
pub struct Sound {
    pub record_flags: ObjectFlags,
    pub id: RefId,
    pub sound: String,
    pub data: SoundData,
}

impl EsmRecord for Sound {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Soun;
}

impl Sound {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut sound = Sound::default();
        sound.record_flags = reader.get_record_flags();

        let mut has_name = false;
        let mut has_data = false;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"NAME" => {
                    // Sub-header is already read by get_or_read_sub_name(), read the refid payload directly[cite: 6, 8]
                    sound.id = reader.read_refid()?;
                    has_name = true;
                }
                b"FNAM" => {
                    sound.sound = reader.read_header_decode_string()?;
                }
                b"DATA" => {
                    reader.read_sub_header()?;
                    sound.data = reader.load()?;
                    has_data = true;
                }
                b"DELE" => {
                    reader.skip_sub_header()?;
                    *is_deleted = true;
                }
                _ => {
                    return Err(reader.fail("Unknown subrecord"));
                }
            }
        }

        if !has_name {
            return Err(reader.fail("Missing NAME subrecord"));
        }
        if !has_data && !*is_deleted {
            return Err(reader.fail("Missing DATA subrecord"));
        }

        Ok(sound)
    }
}

#[derive(Clone, Debug, Default, PartialEq, LoadSave)]
pub struct SoundData {
    pub volume: u8,
    pub min_range: u8,
    pub max_range: u8,
}
