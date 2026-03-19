open! Core

module T = struct
  type t = string [@@deriving sexp, compare]
end

include T
include Comparable.Make (T)

let equal = String.equal
let to_string x = x
let of_string x = x
