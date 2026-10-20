use crate::characters::{
    baron, butler, chef, drunk, empath, fortune_teller, imp, investigator, librarian, mayor, monk,
    poisoner, ravenkeeper, recluse, saint, scarlet_woman, slayer, soldier, spy, undertaker, virgin,
    washerwoman,
};
use crate::script;

pub fn script() -> script::Script {
    script::Script {
        name: "Trouble Brewing",
        characters: vec![
            &washerwoman::Def,
            &librarian::Def,
            &investigator::Def,
            &chef::Def,
            &empath::Def,
            &fortune_teller::Def,
            &undertaker::Def,
            &monk::Def,
            &ravenkeeper::Def,
            &virgin::Def,
            &slayer::Def,
            &soldier::Def,
            &mayor::Def,
            &butler::Def,
            &drunk::Def,
            &recluse::Def,
            &saint::Def,
            &poisoner::Def,
            &spy::Def,
            &scarlet_woman::Def,
            &baron::Def,
            &imp::Def,
        ],
        first_night: vec![
            &poisoner::Def,
            &washerwoman::Def,
            &librarian::Def,
            &investigator::Def,
            &chef::Def,
            &empath::Def,
            &fortune_teller::Def,
            &butler::Def,
            &spy::Def,
        ],
        other_nights: vec![
            &poisoner::Def,
            &monk::Def,
            &imp::Def,
            &ravenkeeper::Def,
            &undertaker::Def,
            &empath::Def,
            &fortune_teller::Def,
            &butler::Def,
            &spy::Def,
        ],
    }
}
