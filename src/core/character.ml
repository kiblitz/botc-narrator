open! Core

module Make (C : Character_intf.Base_S) = struct
  include C

  let to_display t =
    { Char_display.id = C.id; name = C.name; kind = C.kind t; alignment = C.alignment t }
  ;;

  let narrator_pick_from xs =
    let n = List.length xs in
    let%map.Botc_exec i = Botc_exec.narrator_pick (List.init n ~f:Fn.id) in
    let chosen = List.nth_exn xs i in
    let rest = List.filteri xs ~f:(fun j _ -> j <> i) in
    chosen, rest
  ;;

  let rec pick_n n xs =
    if n = 0
    then Botc_exec.return []
    else (
      let%bind.Botc_exec chosen, rest = narrator_pick_from xs in
      let%map.Botc_exec tail = pick_n (n - 1) rest in
      chosen :: tail)
  ;;

  let alive_except state self =
    Game_state.alive_ids state |> List.filter ~f:(fun id -> not (Player_id.equal id self))
  ;;

  let if_alive pid m =
    let%bind.Botc_exec p = Botc_exec.get_player pid in
    Botc_exec.when_ (Player.alive p) m
  ;;
end
