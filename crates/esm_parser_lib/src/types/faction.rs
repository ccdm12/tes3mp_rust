use crate::{
    EsmError, EsmReader, Load, RefId,
    esm_record::EsmRecord,
    types::{enums::RecInt, flags::ObjectFlags},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RankData {
    pub attribute1: i32,
    pub attribute2: i32,
    pub primary_skill: i32,
    pub favoured_skill: i32,
    pub fact_reputation: i32,
}

impl Load for RankData {
    fn load(reader: &mut EsmReader) -> Result<Self, EsmError> {
        let attribute1 = reader.load()?;
        let attribute2 = reader.load()?;
        let primary_skill = reader.load()?;
        let favoured_skill = reader.load()?;
        let fact_reputation = reader.load()?;

        Ok(Self {
            attribute1,
            attribute2,
            primary_skill,
            favoured_skill,
            fact_reputation,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FADTstruct {
    pub attribute: [i32; 2],
    pub rank_data: [RankData; 10],
    pub skills: [i32; 7],
    pub is_hidden: i32,
}

impl FADTstruct {
    pub fn get_skill(&self, index: usize) -> Result<i32, EsmError> {
        if index >= self.skills.len() {
            return Err(EsmError::Custom(format!(
                "Skill index {} out of range",
                index
            )));
        }
        Ok(self.skills[index])
    }
}

impl Load for FADTstruct {
    fn load(reader: &mut EsmReader) -> Result<Self, EsmError> {
        reader.read_sub_header()?;
        let attribute = [reader.load()?, reader.load()?];
        let rank_data = std::array::from_fn(|_| RankData::load(reader).unwrap_or_default());
        let mut skills = [0; 7];
        for skill in skills.iter_mut() {
            *skill = reader.load()?;
        }
        let is_hidden = reader.load()?;

        if is_hidden > 1 {
            return Err(reader.fail("Unknown flag!"));
        }

        Ok(Self {
            attribute,
            rank_data,
            skills,
            is_hidden,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Faction {
    pub record_flags: ObjectFlags,
    pub name: String,
    pub id: RefId,
    pub data: FADTstruct,
    pub reactions: std::collections::HashMap<RefId, i32>,
    pub ranks: [String; 10],
}

impl EsmRecord for Faction {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Fact;
}

impl Faction {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut faction = Faction::default();
        faction.record_flags = reader.get_record_flags();

        let mut rank_counter = 0;
        let mut has_name = false;
        let mut has_data = false;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"NAME" => {
                    faction.id = reader.read_refid()?;
                    has_name = true;
                }
                b"FNAM" => {
                    faction.name = reader.read_header_decode_string()?;
                }
                b"RNAM" => {
                    if rank_counter >= faction.ranks.len() {
                        return Err(reader.fail("Rank out of range"));
                    }
                    faction.ranks[rank_counter] = reader.read_header_decode_string()?;
                    rank_counter += 1;
                }
                b"FADT" => {
                    faction.data = reader.load()?;
                    has_data = true;
                }
                b"ANAM" => {
                    let faction_id = reader.read_refid()?;
                    let reaction: i32 = reader.read_typed_named_header(crate::name4!("INTV"))?;

                    // Prefer the lowest reaction in case a faction is listed multiple times[cite: 27]
                    faction
                        .reactions
                        .entry(faction_id)
                        .and_modify(|r| {
                            if *r > reaction {
                                *r = reaction;
                            }
                        })
                        .or_insert(reaction);
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
            return Err(reader.fail("Missing FADT subrecord"));
        }

        Ok(faction)
    }

    pub fn blank(&mut self) {
        self.record_flags = ObjectFlags::empty();
        self.name.clear();
        self.data.attribute = [0; 2];
        self.data.rank_data = std::array::from_fn(|_| RankData::default());
        self.data.skills = [0; 7];
        self.data.is_hidden = 0;
        self.reactions.clear();
        self.ranks = std::array::from_fn(|_| String::new());
    }
}
