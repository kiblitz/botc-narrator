open! Core

module type S = sig
  val all : (module Character_intf.S) list
  val night_one_order : (module Character_intf.S) list
  val night_subsequent_order : (module Character_intf.S) list
end
