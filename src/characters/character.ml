open! Core

(** Characters that participate in the night phase. Scripts use this type. *)
module type WITH_NIGHT_ACTION = sig
  include Character_intf.S

  val night_action
    :  player_id:Player_id.t
    -> night:int
    -> unit Narrator_monad.t option
end

let ( let* ) = Narrator_monad.( let* )

(** Pick the element at a narrator-chosen index, returning it and the rest. *)
let narrator_pick_from xs =
  let n = List.length xs in
  let* i = Narrator_monad.narrator_pick (List.init n ~f:Fn.id) in
  let chosen = List.nth_exn xs i in
  let rest   = List.filteri xs ~f:(fun j _ -> j <> i) in
  Narrator_monad.return (chosen, rest)

(** Pick [n] distinct elements via narrator. *)
let rec pick_n n xs =
  if n = 0 then Narrator_monad.return []
  else
    let* (chosen, rest) = narrator_pick_from xs in
    let* tail           = pick_n (n - 1) rest in
    Narrator_monad.return (chosen :: tail)

let alive_except state self =
  Game_state.alive_ids state
  |> List.filter ~f:(fun id -> not (Player_id.equal id self))

let if_alive pid m =
  let* p = Narrator_monad.get_player pid in
  Narrator_monad.when_ p.Player.alive m
