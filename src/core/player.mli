open! Core

type t =
  { id             : Player_id.t
  ; name           : string
  ; character      : Character_intf.t
  ; shown_character: Character_intf.t
  ; alive          : bool
  ; has_ghost_vote : bool
  }

val create : id:Player_id.t -> name:string -> character:Character_intf.t -> t

val is_evil      : t -> bool
val is_good      : t -> bool
val character_id : t -> string
