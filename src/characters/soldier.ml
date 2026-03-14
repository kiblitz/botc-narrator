open! Core

let id        = "soldier"
let name      = "Soldier"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good
let night_action ~player_id:_ ~night:_ = None
