open! Core

type t =
  { id : Player_id.t
  ; character : Char_display.t
  ; shown_character : Char_display.t
  ; alive : bool
  ; has_ghost_vote : bool
  }
[@@deriving fields ~getters, sexp_of]

let create ~id ~character =
  { id; character; shown_character = character; alive = true; has_ghost_vote = false }
;;

let name p = Player_id.to_string (id p)
let character_id p = Char_display.id (character p)
let is_evil p = Char_display.is_evil (character p)
let is_good p = Char_display.is_good (character p)
