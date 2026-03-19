open! Core
module Kind = Character0.Kind
module Alignment = Character0.Alignment

module Packed : sig
  type t =
    | T :
        { state : 'a
        ; character : (module Character_intf.S with type t = 'a)
        }
        -> t
end

module Make (_ : Character_intf.Input_S) : Character_intf.Base_S
