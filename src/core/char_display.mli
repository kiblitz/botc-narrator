open! Core

type t =
  { id : string
  ; name : string
  ; kind : Kind.t
  ; alignment : Alignment.t
  }
[@@deriving fields ~getters, sexp_of]

val is_townsfolk : t -> bool
val is_outsider : t -> bool
val is_minion : t -> bool
val is_demon : t -> bool
val is_evil : t -> bool
val is_good : t -> bool
