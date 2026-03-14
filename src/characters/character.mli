open! Core

module type WITH_NIGHT_ACTION = sig
  include Character_intf.S

  val night_action
    :  player_id:Player_id.t
    -> night:int
    -> unit Narrator_monad.t option
end

val narrator_pick_from : 'a list -> ('a * 'a list) Narrator_monad.t
val pick_n             : int -> 'a list -> 'a list Narrator_monad.t
val alive_except       : Game_state.t -> Player_id.t -> Player_id.t list
val if_alive           : Player_id.t -> unit Narrator_monad.t -> unit Narrator_monad.t
