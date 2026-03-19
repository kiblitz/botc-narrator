open! Core

(** GADT of narrator instructions. The type parameter is the return type. *)
module Instr = struct
  type _ t =
    | Wake : Player_id.t -> unit t
    | Sleep : Player_id.t -> unit t
    | Tell : Player_id.t * string -> unit t
    (** Narrator tells a player something (one-way information). *)
    | Ask : Player_id.t * string * Player_id.t list -> Player_id.t t
    (** Narrator asks a player a question; they point at one of the candidates. *)
    | Narrator_pick : string * 'a list -> 'a t
    (** Narrator freely chooses one element (their discretion). *)
    | Log : string -> unit t
    | Get_state : Game_state.t t
    | Set_state : Game_state.t -> unit t
end

(** Free monad over [Instr.t].

    The [Both] constructor captures two independent computations that don't
    depend on each other's results. A synchronous engine runs them sequentially,
    but an async engine can run them in parallel. Use [let%map x = a and y = b]
    to produce [Both] nodes. *)
type 'a t =
  | Return : 'a -> 'a t
  | Step : 'b Instr.t * ('b -> 'a t) -> 'a t
  | Both : 'b t * 'c t * ('b * 'c -> 'a t) -> 'a t

let return x = Return x

let rec bind : type a b. a t -> f:(a -> b t) -> b t =
  fun m ~f ->
  match m with
  | Return x -> f x
  | Step (i, k) -> Step (i, fun x -> bind (k x) ~f)
  | Both (l, r, k) -> Both (l, r, fun pair -> bind (k pair) ~f)
;;

let map m ~f = bind m ~f:(fun x -> return (f x))
let both l r = Both (l, r, fun p -> Return p)

include (
  Monad.Make (struct
    type nonrec 'a t = 'a t

    let return = return
    let bind = bind
    let map = `Custom map
  end) :
    Monad.S with type 'a t := 'a t)

(* Override Let_syntax so that [let%map ... and ...] uses our parallel [both]
   instead of the sequential one derived by [Monad.Make]. *)
module Let_syntax = struct
  let return = return
  let ( >>= ) = ( >>= )

  module Let_syntax = struct
    let return = return
    let bind = bind
    let map = map
    let both = both

    module Open_on_rhs = struct end
  end
end

let lift i = Step (i, return)
let wake p = lift (Instr.Wake p)
let sleep p = lift (Instr.Sleep p)
let tell p msg = lift (Instr.Tell (p, msg))
let ask p question cs = lift (Instr.Ask (p, question, cs))
let pick_one prompt xs = lift (Instr.Narrator_pick (prompt, xs))
let log s = lift (Instr.Log s)
let get_state = lift Instr.Get_state
let set_state s = lift (Instr.Set_state s)
let modify_state f = bind get_state ~f:(fun s -> set_state (f s))
let when_ pred m = if pred then m else return ()

let iter xs ~f =
  List.fold xs ~init:(return ()) ~f:(fun acc x -> bind acc ~f:(fun () -> f x))
;;

let rec narrator_pick prompt xs ~pick_count =
  if pick_count = 0
  then return []
  else (
    let n = List.length xs in
    bind
      (pick_one prompt (List.init n ~f:Fn.id))
      ~f:(fun i ->
        let chosen = List.nth_exn xs i in
        let rest = List.filteri xs ~f:(fun j _ -> j <> i) in
        bind
          (narrator_pick prompt rest ~pick_count:(pick_count - 1))
          ~f:(fun tail -> return (chosen :: tail))))
;;

module type Engine_S = Botc_exec_intf.Engine_S

let rec has_visible_effect : type a. a t -> bool = function
  | Return _ -> false
  | Step (Instr.Get_state, _) -> false
  | Step (_, _) -> true
  | Both (l, r, _) -> has_visible_effect l || has_visible_effect r
;;

let run (type a) (module I : Engine_S) (initial_state : Game_state.t) (m : a t)
  : a * Game_state.t
  =
  let rec go : type b. Game_state.t -> b t -> b * Game_state.t =
    fun state -> function
      | Return x -> x, state
      | Step (Instr.Wake p, k) ->
        I.wake p;
        go state (k ())
      | Step (Instr.Sleep p, k) ->
        I.sleep p;
        go state (k ())
      | Step (Instr.Tell (p, msg), k) ->
        I.tell p msg;
        go state (k ())
      | Step (Instr.Ask (p, q, cs), k) -> go state (k (I.ask p q cs))
      | Step (Instr.Narrator_pick (prompt, xs), k) ->
        go state (k (I.narrator_pick prompt xs))
      | Step (Instr.Log s, k) ->
        I.log s;
        go state (k ())
      | Step (Instr.Get_state, k) -> go state (k state)
      | Step (Instr.Set_state s, k) -> go s (k ())
      | Both (l, r, k) ->
        let log_parallel = has_visible_effect l && has_visible_effect r in
        if log_parallel then I.log "[parallel 0]";
        let lv, state = go state l in
        if log_parallel then I.log "[parallel 1]";
        let rv, state = go state r in
        go state (k (lv, rv))
  in
  go initial_state m
;;
