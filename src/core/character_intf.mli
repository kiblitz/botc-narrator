open! Core

type kind      = Townsfolk | Outsider | Minion | Demon [@@deriving sexp, equal, compare]
type alignment = Good      | Evil                      [@@deriving sexp, equal, compare]

module type S = sig
  val id        : string
  val name      : string
  val kind      : kind
  val alignment : alignment
end

(** A first-class character module. *)
type t = (module S)

val id        : t -> string
val name      : t -> string
val kind      : t -> kind
val alignment : t -> alignment

val equal   : t -> t -> bool
val compare : t -> t -> int

val is_townsfolk : t -> bool
val is_outsider  : t -> bool
val is_minion    : t -> bool
val is_demon     : t -> bool
val is_evil      : t -> bool
val is_good      : t -> bool
