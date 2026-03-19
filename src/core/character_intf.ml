open! Core

(** The minimal input to the [Make] functor. [type t] is abstract and
    character-specific, allowing each character to carry its own state. *)
module type Base_S = sig
  val id : string
  val name : string

  type t

  val t : t
  val kind : t -> Kind.t
  val alignment : t -> Alignment.t
end

(** Full character interface produced by [Make] and exposed by every character's [.mli]. *)
module type S = sig
  include Base_S

  val to_display : t -> Char_display.t
  val night_action : player_id:Player_id.t -> night:int -> unit Botc_exec.t option
  val day_action : player_id:Player_id.t -> unit Botc_exec.t option
  val on_setup : player_id:Player_id.t -> unit Botc_exec.t option

  val on_nominated
    :  player_id:Player_id.t
    -> nominator:Player_id.t
    -> unit Botc_exec.t option

  val on_executed : player_id:Player_id.t -> unit Botc_exec.t option
  val on_night_kill : player_id:Player_id.t -> unit Botc_exec.t option
  val narrator_pick_from : 'a list -> ('a * 'a list) Botc_exec.t
  val pick_n : int -> 'a list -> 'a list Botc_exec.t
  val alive_except : Game_state.t -> Player_id.t -> Player_id.t list
  val if_alive : Player_id.t -> unit Botc_exec.t -> unit Botc_exec.t
end

module type Character = sig
  module Make (_ : Base_S) : S
end
