open! Core

let id        = "slayer"
let name      = "Slayer"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good
let night_action ~player_id:_ ~night:_ = None
