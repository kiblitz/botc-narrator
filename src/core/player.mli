open! Core

type t =
  { id : Player_id.t
  ; character : Char_display.t
  ; shown_character : Char_display.t
  ; alive : bool
  ; has_ghost_vote : bool
  }
[@@deriving fields ~getters, sexp_of]

val create : id:Player_id.t -> character:Char_display.t -> t
val name : t -> string
val is_evil : t -> bool
val is_good : t -> bool
val character_id : t -> string
