use crate::{
    EsmError, EsmReader, RefId,
    esm_reader::ConsumptionType,
    esm_record::EsmRecord,
    types::{enums::RecInt, flags::ObjectFlags},
};
use esm_parser_macros::LoadSave;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Region {
    pub data: WEATstruct,
    pub map_color: i32,
    pub record_flags: ObjectFlags,
    pub id: RefId,
    pub sleep_list: RefId,
    pub name: String,
    pub sound_list: Vec<SoundRef>,
}

impl EsmRecord for Region {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Regn;
}

impl Region {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut region = Region::default();
        region.record_flags = reader.get_record_flags();

        let mut has_name = false;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"NAME" => {
                    region.id = reader.read_refid()?;
                    has_name = true;
                }
                b"FNAM" => {
                    region.name = reader.read_header_decode_string()?;
                }
                b"WEAT" => {
                    reader.read_sub_header()?;
                    let sub_size = reader.get_sub_size() as usize;

                    if sub_size == region.data.m_probabilities.len() {
                        reader.read_exact(
                            &mut region.data.m_probabilities,
                            ConsumptionType::SubrecordPayload,
                        )?;
                    } else if sub_size == region.data.m_probabilities.len() - 2 {
                        region.data.m_probabilities.fill(0);
                        reader.read_exact(
                            &mut region.data.m_probabilities[..sub_size],
                            ConsumptionType::SubrecordPayload,
                        )?;
                    } else {
                        return Err(reader.fail("Don't know what to do in this version"));
                    }
                }
                b"BNAM" => {
                    region.sleep_list = reader.read_refid()?;
                }
                b"CNAM" => {
                    region.map_color = reader.read_typed_header()?;
                }
                b"SNAM" => {
                    reader.read_sub_header()?;
                    let mut sound_ref = SoundRef::default();

                    // SNAM packs a maybe-fixed refid (32 bytes max in older versions or variant) + 1 byte chance
                    sound_ref.sound = reader.get_maybe_fixed_refid_sized(32)?;
                    sound_ref.chance = reader.read_typed(ConsumptionType::SubrecordPayload)?;

                    region.sound_list.push(sound_ref);
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

        Ok(region)
    }
}

#[derive(Clone, Debug, Default, PartialEq, LoadSave)]
pub struct WEATstruct {
    pub m_probabilities: [u8; 10],
}

#[derive(Clone, Debug, Default, PartialEq, LoadSave)]
pub struct SoundRef {
    pub sound: RefId,
    pub chance: u8,
}
