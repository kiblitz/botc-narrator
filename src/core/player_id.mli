open! Core

type t [@@deriving sexp]

include Comparable.S with type t := t

val equal : t -> t -> bool
val to_string : t -> string
val of_string : string -> t
