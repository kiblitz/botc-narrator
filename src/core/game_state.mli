open! Core

type phase =
  | Setup
  | Day   of { number : int }
  | Night of { number : int }
[@@deriving sexp]

type t =
  { seat_order     : Player_id.t list
  ; players        : Player.t Player_id.Map.t
  ; phase          : phase
  ; poisoned       : Player_id.t option
  ; monk_protected : Player_id.t option
  ; night_deaths   : Player_id.t list
  ; last_execution : Player_id.t option
  ; imp_starpass   : bool
  }

val create : Player_id.t list -> Player.t list -> t

val seated_players : t -> Player.t list
val alive_players  : t -> Player.t list
val alive_ids      : t -> Player_id.t list

(** Find the first player whose actual character has the given id. *)
val find_character_id : t -> string -> Player.t option

val is_poisoned : t -> Player_id.t -> bool

val kill               : t -> Player_id.t -> t
val set_poisoned       : t -> Player_id.t -> t
val clear_poisoned     : t -> t
val set_monk_protected : t -> Player_id.t -> t
val clear_monk_protected : t -> t

val begin_night : t -> int -> t
val begin_day   : t -> int -> t

val character_ids_in_play       : t -> string list
val good_char_names_not_in_play : t -> Character_intf.t list -> string list
