open! Core

(** GADT of narrator instructions. The type parameter is the return type. *)
type _ instr =
  | Wake : Player_id.t -> unit instr
  | Sleep : Player_id.t -> unit instr
  | Tell : Player_id.t * string -> unit instr
  (** Narrator tells a player something (one-way information). *)
  | Ask : Player_id.t * string * Player_id.t list -> Player_id.t instr
  (** Narrator asks a player a question; they point at one of the candidates. *)
  | Narrator_pick : 'a list -> 'a instr
  (** Narrator freely chooses one element (their discretion). *)
  | Log : string -> unit instr
  | Get_state : Game_state.t instr
  | Set_state : Game_state.t -> unit instr

(** Free monad over [instr]. *)
type 'a t =
  | Return : 'a -> 'a t
  | Step : 'b instr * ('b -> 'a t) -> 'a t

let return x = Return x

let rec bind : type a b. a t -> f:(a -> b t) -> b t =
  fun m ~f ->
  match m with
  | Return x -> f x
  | Step (i, k) -> Step (i, fun x -> bind (k x) ~f)
;;

include Monad.Make (struct
    type nonrec 'a t = 'a t

    let return = return
    let bind = bind
    let map = `Custom (fun m ~f -> bind m ~f:(fun x -> return (f x)))
  end)

let lift i = Step (i, return)
let wake p = lift (Wake p)
let sleep p = lift (Sleep p)
let tell p msg = lift (Tell (p, msg))
let ask p question cs = lift (Ask (p, question, cs))
let narrator_pick xs = lift (Narrator_pick xs)
let log s = lift (Log s)
let get_state = lift Get_state
let set_state s = lift (Set_state s)
let modify_state f = bind get_state ~f:(fun s -> set_state (f s))
let when_ pred m = if pred then m else return ()

let get_player id =
  bind get_state ~f:(fun s -> return (Map.find_exn (Game_state.players s) id))
;;

(** Swap implementations to mock player interactions in tests. *)
module type Interp_S = sig
  val wake : Player_id.t -> unit
  val sleep : Player_id.t -> unit
  val tell : Player_id.t -> string -> unit
  val ask : Player_id.t -> string -> Player_id.t list -> Player_id.t
  val narrator_pick : 'a list -> 'a
  val log : string -> unit
end

let run (type a) (module I : Interp_S) (initial_state : Game_state.t) (m : a t)
  : a * Game_state.t
  =
  let rec go : type b. Game_state.t -> b t -> b * Game_state.t =
    fun state -> function
      | Return x -> x, state
      | Step (Wake p, k) ->
        I.wake p;
        go state (k ())
      | Step (Sleep p, k) ->
        I.sleep p;
        go state (k ())
      | Step (Tell (p, msg), k) ->
        I.tell p msg;
        go state (k ())
      | Step (Ask (p, q, cs), k) -> go state (k (I.ask p q cs))
      | Step (Narrator_pick xs, k) -> go state (k (I.narrator_pick xs))
      | Step (Log s, k) ->
        I.log s;
        go state (k ())
      | Step (Get_state, k) -> go state (k state)
      | Step (Set_state s, k) -> go s (k ())
  in
  go initial_state m
;;
