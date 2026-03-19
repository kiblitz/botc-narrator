open! Core

module Phase : sig
  type t =
    | Setup
    | Day of { number : int }
    | Night of { number : int }
  [@@deriving sexp]
end

type t [@@deriving sexp_of]

val create : Player_id.t list -> Player.t list -> t

(** Getters *)

val phase : t -> Phase.t
val players : t -> Player.t Player_id.Map.t
val night_deaths : t -> Player_id.t list
val last_execution : t -> Player_id.t option
val monk_protected : t -> Player_id.t option
val seated_players : t -> Player.t list
val alive_players : t -> Player.t list
val alive_ids : t -> Player_id.t list

(** Find the first player whose actual character has the given id. *)
val find_character_id : t -> string -> Player.t option

(** Advance phase: Setup → Night 1 → Day 1 → Night 2 → Day 2 → ... *)
val next_phase : t -> t

(** Mutations *)

val is_poisoned : t -> Player_id.t -> bool
val kill : t -> Player_id.t -> t
val set_players : t -> Player.t Player_id.Map.t -> t
val set_poisoned : t -> Player_id.t -> t
val clear_poisoned : t -> t
val set_monk_protected : t -> Player_id.t -> t
val clear_monk_protected : t -> t
val use_day_ability : t -> Player_id.t -> t
val has_used_day_ability : t -> Player_id.t -> bool
val character_ids_in_play : t -> string list
val good_chars_not_in_play : t -> Char_display.t list -> Char_display.t list
