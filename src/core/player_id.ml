open! Core

module T = struct
  type t = int [@@deriving sexp, compare]
end

include T
include Comparable.Make (T)

let equal a b = T.compare a b = 0
let of_int x  = x
let to_int x  = x
let to_string = Int.to_string
