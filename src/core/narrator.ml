open! Core

let minion_info () =
  let%bind.Botc_exec state = Botc_exec.get_state in
  let minions =
    List.filter (Game_state.alive_players state) ~f:(fun p ->
      Char_display.is_minion (Player.character p))
  in
  if List.is_empty minions
  then Botc_exec.return ()
  else (
    let evil_names =
      List.filter_map (Game_state.alive_players state) ~f:(fun p ->
        if Char_display.is_evil (Player.character p) then Some (Player.name p) else None)
    in
    let%bind.Botc_exec () =
      List.fold minions ~init:(Botc_exec.return ()) ~f:(fun acc p ->
        let%bind.Botc_exec () = acc in
        Botc_exec.wake (Player.id p))
    in
    let%bind.Botc_exec () =
      List.fold minions ~init:(Botc_exec.return ()) ~f:(fun acc p ->
        let%bind.Botc_exec () = acc in
        Botc_exec.tell (Player.id p) ("Evil: " ^ String.concat evil_names ~sep:", "))
    in
    List.fold minions ~init:(Botc_exec.return ()) ~f:(fun acc p ->
      let%bind.Botc_exec () = acc in
      Botc_exec.sleep (Player.id p)))
;;

let demon_info all () =
  let%bind.Botc_exec state = Botc_exec.get_state in
  let demon =
    List.find (Game_state.alive_players state) ~f:(fun p ->
      Char_display.is_demon (Player.character p))
  in
  match demon with
  | None -> Botc_exec.return ()
  | Some d ->
    let not_in_play =
      Game_state.good_chars_not_in_play
        state
        (List.map all ~f:(fun (module C : Character_intf.S) -> C.to_display C.t))
    in
    let count = min 3 (List.length not_in_play) in
    let rec pick_n n xs =
      if n = 0
      then Botc_exec.return []
      else (
        let n_xs = List.length xs in
        let%bind.Botc_exec i = Botc_exec.narrator_pick (List.init n_xs ~f:Fn.id) in
        let chosen = List.nth_exn xs i in
        let rest = List.filteri xs ~f:(fun j _ -> j <> i) in
        let%map.Botc_exec tail = pick_n (n - 1) rest in
        chosen :: tail)
    in
    let%bind.Botc_exec () = Botc_exec.wake (Player.id d)
    and bluffs = pick_n count not_in_play in
    let%bind.Botc_exec () =
      Botc_exec.tell
        (Player.id d)
        ("Bluffs: " ^ (List.map bluffs ~f:Char_display.name |> String.concat ~sep:", "))
    in
    Botc_exec.sleep (Player.id d)
;;

let run_order order night =
  List.fold order ~init:(Botc_exec.return ()) ~f:(fun acc (module C : Character_intf.S) ->
    let%bind.Botc_exec () = acc in
    let%bind.Botc_exec state = Botc_exec.get_state in
    match Game_state.find_character_id state C.id with
    | None -> Botc_exec.return ()
    | Some player ->
      (match C.night_action ~player_id:(Player.id player) ~night with
       | None -> Botc_exec.return ()
       | Some m -> m))
;;

let run_night (module S : Script_intf.S) () =
  let%bind.Botc_exec state = Botc_exec.get_state in
  match Game_state.phase state with
  | Game_state.Phase.Night { number = 1 } ->
    let%bind.Botc_exec () = minion_info () in
    let%bind.Botc_exec () = demon_info S.all () in
    run_order S.night_one_order 1
  | Game_state.Phase.Night { number = n } -> run_order S.night_subsequent_order n
  | _ -> Botc_exec.return ()
;;
