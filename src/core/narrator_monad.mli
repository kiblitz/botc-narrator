open! Core

type night_info =
  | Two_players_one_char of Player_id.t * Player_id.t * string
  | No_outsiders
  | Number          of int
  | Yes_or_no       of bool
  | Character_revealed of string
  | Grimoire        of (Player_id.t * string) list
  | Evil_players    of Player_id.t list
  | Demon_bluffs    of string list

(** Abstract free monad type.  Build values using the combinators below. *)
type 'a t

val return : 'a -> 'a t
val bind   : 'a t -> ('a -> 'b t) -> 'b t

val ( >>= ) : 'a t -> ('a -> 'b t) -> 'b t
val ( let* ) : 'a t -> ('a -> 'b t) -> 'b t
val ( >>  ) : unit t -> unit t -> unit t

val wake             : Player_id.t -> unit t
val sleep            : Player_id.t -> unit t
val show_info        : Player_id.t -> night_info -> unit t
val player_points    : Player_id.t -> Player_id.t list -> Player_id.t t
val player_points_two: Player_id.t -> Player_id.t list -> (Player_id.t * Player_id.t) t
val narrator_pick    : 'a list -> 'a t
val log              : string -> unit t
val get_state        : Game_state.t t
val set_state        : Game_state.t -> unit t
val modify_state     : (Game_state.t -> Game_state.t) -> unit t
val when_            : bool -> unit t -> unit t
val get_player       : Player_id.t -> Player.t t

module type INTERP = sig
  val wake             : Player_id.t -> unit
  val sleep            : Player_id.t -> unit
  val show_info        : Player_id.t -> night_info -> unit
  val player_points    : Player_id.t -> Player_id.t list -> Player_id.t
  val player_points_two: Player_id.t -> Player_id.t list -> Player_id.t * Player_id.t
  val narrator_pick    : 'a list -> 'a
  val log              : string -> unit
end

val run : (module INTERP) -> Game_state.t -> 'a t -> 'a * Game_state.t
