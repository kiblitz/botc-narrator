open! Core

module Phase : sig
  type t =
    | Setup
    | Day of { number : int }
    | Night of { number : int }
  [@@deriving sexp]
end

module Player_spec : sig
  type t =
    { id : Player_id.t
    ; character_id : string
    ; character_name : string
    ; kind : Character0.Kind.t
    ; alignment : Character0.Alignment.t
    }
end

type t [@@deriving sexp_of]

val create : Player_id.t list -> Player_spec.t list -> t

(** Game metadata *)

val phase : t -> Phase.t
val night_deaths : t -> Player_id.t list
val last_execution : t -> Player_id.t option
val seat_order : t -> Player_id.t list

(** Player queries *)

val alive_ids : t -> Player_id.t list
val is_alive : t -> Player_id.t -> bool
val character_id : t -> Player_id.t -> string
val character_name : t -> Player_id.t -> string
val kind : t -> Player_id.t -> Character0.Kind.t
val alignment : t -> Player_id.t -> Character0.Alignment.t
val is_evil : t -> Player_id.t -> bool
val is_good : t -> Player_id.t -> bool
val is_poisoned : t -> Player_id.t -> bool
val monk_protected : t -> Player_id.t -> bool
val has_used_day_ability : t -> Player_id.t -> bool

(** Find the first player whose actual character has the given id. *)
val find_character_id : t -> string -> Player_id.t option

val character_ids_in_play : t -> string list

(** Advance phase: Setup -> Night 1 -> Day 1 -> Night 2 -> Day 2 -> ... *)
val next_phase : t -> t

(** Mutations *)

val kill : t -> Player_id.t -> t
val set_poisoned : t -> Player_id.t -> t
val clear_poisoned : t -> t
val set_monk_protected : t -> Player_id.t -> t
val clear_monk_protected : t -> t
val use_day_ability : t -> Player_id.t -> t

val transform_into
  :  t
  -> Player_id.t
  -> character_id:string
  -> character_name:string
  -> kind:Character0.Kind.t
  -> alignment:Character0.Alignment.t
  -> t
