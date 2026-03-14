open! Core

type t =
  { id             : Player_id.t
  ; name           : string
  (** The character they actually are.  The Drunk is [Drunk], but
      [shown_character] is the Townsfolk they believe themselves to be. *)
  ; character      : Character_intf.t
  (** What this player was told they are (differs only for the Drunk). *)
  ; shown_character: Character_intf.t
  ; alive          : bool
  (** Dead players retain one ghost vote. *)
  ; has_ghost_vote : bool
  }

let create ~id ~name ~character =
  { id; name; character; shown_character = character; alive = true; has_ghost_vote = false }

let is_evil      p = Character_intf.is_evil p.character
let is_good      p = Character_intf.is_good p.character
let character_id p = Character_intf.id      p.character
