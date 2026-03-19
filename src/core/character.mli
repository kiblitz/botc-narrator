open! Core

module Make (_ : Character_intf.Base_S) : sig
  include Character_intf.Base_S

  val to_display : t -> Char_display.t
  val narrator_pick_from : 'a list -> ('a * 'a list) Botc_exec.t
  val pick_n : int -> 'a list -> 'a list Botc_exec.t
  val alive_except : Game_state.t -> Player_id.t -> Player_id.t list
  val if_alive : Player_id.t -> unit Botc_exec.t -> unit Botc_exec.t
end
