open! Core

let id        = "mayor"
let name      = "Mayor"
let kind      = Character_intf.Townsfolk
let alignment = Character_intf.Good
let night_action ~player_id:_ ~night:_ = None
