open! Core
open Botc_narrator_lib

val p : string -> Player_id.t

val mk_players : (string * (module Character_intf.S)) list -> Player.t list
val mk_state : Player.t list -> Game_state.t
val night_state : Player.t list -> Game_state.t
val day_state : Player.t list -> Game_state.t

val run
  :  players:Player.t list
  -> ?silent:bool
  -> ?responses:Player_id.t list
  -> action:unit Botc_exec.t
  -> Game_state.t
  -> Game_state.t

(** Unwrap a character's night action, converting the player_id from string. *)
val night_action
  :  (player_id:Player_id.t -> night:int -> unit Botc_exec.t option)
  -> night:int
  -> player_id:string
  -> unit Botc_exec.t

(** Unwrap a character's day action, converting the player_id from string. *)
val day_action
  :  (player_id:Player_id.t -> unit Botc_exec.t option)
  -> player_id:string
  -> unit Botc_exec.t

val print_grimoire : Game_state.t -> unit
