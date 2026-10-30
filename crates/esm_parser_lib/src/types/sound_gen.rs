use crate::{
    EsmError, EsmReader, RefId,
    esm_record::EsmRecord,
    types::{
        enums::{RecInt, SoundGenType},
        flags::ObjectFlags,
    },
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SoundGenerator {
    pub sound_gen_type: SoundGenType,
    pub record_flags: ObjectFlags,
    pub id: RefId,
    pub creature: RefId,
    pub sound: RefId,
}

impl EsmRecord for SoundGenerator {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Sndg;
}

impl SoundGenerator {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut sound_gen = SoundGenerator::default();
        sound_gen.record_flags = reader.get_record_flags();

        let mut has_name = false;
        let mut has_data = false;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"NAME" => {
                    sound_gen.id = reader.read_refid()?;
                    has_name = true;
                }
                b"DATA" => {
                    // read_typed_header_load handles sub_header reading internally
                    sound_gen.sound_gen_type = reader.read_typed_header_load()?;
                    has_data = true;
                }
                b"CNAM" => {
                    sound_gen.creature = reader.read_refid()?;
                }
                b"SNAM" => {
                    sound_gen.sound = reader.read_refid()?;
                }
                b"DELE" => {
                    reader.skip_sub_header()?;
                    *is_deleted = true;
                }
                _ => {
                    return Err(reader.fail(&format!("Unknown subrecord: {:?}", sub_name)));
                }
            }
        }

        if !has_name {
            return Err(reader.fail("Missing NAME subrecord"));
        }
        if !has_data && !*is_deleted {
            return Err(reader.fail("Missing DATA subrecord"));
        }

        Ok(sound_gen)
    }
}
