open! Core

type t [@@deriving sexp]

include Comparable.S with type t := t

val equal     : t -> t -> bool
val of_int    : int -> t
val to_int    : t -> int
val to_string : t -> string
