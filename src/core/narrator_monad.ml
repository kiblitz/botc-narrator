open! Core

(** Information shown to a player during the night.
    Character names are plain strings so this module stays independent of
    the character module hierarchy. *)
type night_info =
  | Two_players_one_char of Player_id.t * Player_id.t * string
      (** Washerwoman / Librarian / Investigator:
          two players shown, one of them IS the named character. *)
  | No_outsiders
      (** Librarian: no outsiders are in play. *)
  | Number          of int
      (** Chef or Empath. *)
  | Yes_or_no       of bool
      (** Fortune Teller. *)
  | Character_revealed of string
      (** Undertaker or Ravenkeeper: a character name. *)
  | Grimoire        of (Player_id.t * string) list
      (** Spy: every seated player paired with their character name. *)
  | Evil_players    of Player_id.t list
      (** Minion info: the full evil team. *)
  | Demon_bluffs    of string list
      (** Demon info: up to three not-in-play good character names. *)

(** GADT of narrator instructions.  The type parameter is the return type. *)
type _ instr =
  | Wake              : Player_id.t -> unit instr
  | Sleep             : Player_id.t -> unit instr
  | Show_info         : Player_id.t * night_info -> unit instr
  | Player_points     : Player_id.t * Player_id.t list -> Player_id.t instr
      (** Ask [player] to point at exactly one of [candidates]. *)
  | Player_points_two : Player_id.t * Player_id.t list -> (Player_id.t * Player_id.t) instr
      (** Ask [player] to point at exactly two distinct players. *)
  | Narrator_pick     : 'a list -> 'a instr
      (** Narrator freely chooses one element (their discretion). *)
  | Log               : string -> unit instr
  | Get_state         : Game_state.t instr
  | Set_state         : Game_state.t -> unit instr

(** Free monad over [instr]. *)
type 'a t =
  | Pure : 'a -> 'a t
  | Free : 'b instr * ('b -> 'a t) -> 'a t

let return x = Pure x

let rec bind : type a b. a t -> (a -> b t) -> b t = fun m f ->
  match m with
  | Pure x      -> f x
  | Free (i, k) -> Free (i, fun x -> bind (k x) f)

let ( >>= ) = bind
let ( let* ) = bind
let ( >>  ) m n = m >>= fun () -> n

let lift i = Free (i, return)

let wake p                 = lift (Wake p)
let sleep p                = lift (Sleep p)
let show_info p info       = lift (Show_info (p, info))
let player_points p cs     = lift (Player_points (p, cs))
let player_points_two p cs = lift (Player_points_two (p, cs))
let narrator_pick xs       = lift (Narrator_pick xs)
let log s                  = lift (Log s)
let get_state              = lift Get_state
let set_state s            = lift (Set_state s)

let modify_state f =
  let* s = get_state in
  set_state (f s)

let when_ pred m = if pred then m else return ()

let get_player id =
  let* s = get_state in
  return (Map.find_exn s.Game_state.players id)

(** Swap implementations to mock player interactions in tests. *)
module type INTERP = sig
  val wake             : Player_id.t -> unit
  val sleep            : Player_id.t -> unit
  val show_info        : Player_id.t -> night_info -> unit
  val player_points    : Player_id.t -> Player_id.t list -> Player_id.t
  val player_points_two: Player_id.t -> Player_id.t list -> Player_id.t * Player_id.t
  val narrator_pick    : 'a list -> 'a
  val log              : string -> unit
end

let run (type a) (module I : INTERP) (initial_state : Game_state.t) (m : a t)
    : a * Game_state.t =
  let rec go : type b. Game_state.t -> b t -> b * Game_state.t =
    fun state -> function
    | Pure x                         -> (x, state)
    | Free (Wake p,                  k) -> I.wake p;                        go state (k ())
    | Free (Sleep p,                 k) -> I.sleep p;                       go state (k ())
    | Free (Show_info (p, i),        k) -> I.show_info p i;                 go state (k ())
    | Free (Player_points (p, cs),   k) -> go state (k (I.player_points p cs))
    | Free (Player_points_two (p,cs),k) -> go state (k (I.player_points_two p cs))
    | Free (Narrator_pick xs,        k) -> go state (k (I.narrator_pick xs))
    | Free (Log s,                   k) -> I.log s;                         go state (k ())
    | Free (Get_state,               k) -> go state (k state)
    | Free (Set_state s,             k) ->                                  go s     (k ())
  in
  go initial_state m
