open! Core

module type Input_S = sig
  val id : string
  val name : string

  type t

  val t : t
  val kind : t -> Character0.Kind.t
  val alignment : t -> Character0.Alignment.t
end

module type Base_S = sig
  include Input_S

  val narrator_pick : string -> 'a list -> pick_count:int -> 'a list Botc_exec.t
  val alive_except : Game_state.t -> Player_id.t -> Player_id.t list
  val if_alive : Player_id.t -> unit Botc_exec.t -> unit Botc_exec.t
end

module type S = sig
  include Base_S

  val night_action : player_id:Player_id.t -> night:int -> unit Botc_exec.t option
  val day_action : player_id:Player_id.t -> unit Botc_exec.t option
  val on_setup : player_id:Player_id.t -> unit Botc_exec.t option

  val on_nominated
    :  player_id:Player_id.t
    -> nominator:Player_id.t
    -> unit Botc_exec.t option

  val on_executed : player_id:Player_id.t -> unit Botc_exec.t option
  val on_night_kill : player_id:Player_id.t -> unit Botc_exec.t option
end
