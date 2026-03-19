open! Core

let minion_info () =
  let%bind.Botc_exec state = Botc_exec.get_state in
  let minions =
    List.filter (Game_state.alive_ids state) ~f:(fun id ->
      Character.Kind.equal (Game_state.kind state id) Character.Kind.Minion)
  in
  if List.is_empty minions
  then Botc_exec.return ()
  else (
    let evil_names =
      List.filter_map (Game_state.alive_ids state) ~f:(fun id ->
        if
          Character.Alignment.equal
            (Game_state.alignment state id)
            Character.Alignment.Evil
        then Some (Player_id.to_string id)
        else None)
    in
    let%bind.Botc_exec () = Botc_exec.iter minions ~f:(fun id -> Botc_exec.wake id) in
    let%bind.Botc_exec () =
      Botc_exec.iter minions ~f:(fun id ->
        Botc_exec.tell id ("Evil: " ^ String.concat evil_names ~sep:", "))
    in
    Botc_exec.iter minions ~f:(fun id -> Botc_exec.sleep id))
;;

let demon_info all () =
  let%bind.Botc_exec state = Botc_exec.get_state in
  let demon =
    List.find (Game_state.alive_ids state) ~f:(fun id ->
      Character.Kind.equal (Game_state.kind state id) Character.Kind.Demon)
  in
  match demon with
  | None -> Botc_exec.return ()
  | Some d ->
    let in_play = Game_state.character_ids_in_play state in
    let not_in_play =
      List.filter_map all ~f:(fun (module C : Character_intf.S) ->
        if
          Character.Alignment.equal (C.alignment C.t) Character.Alignment.Good
          && not (List.mem in_play C.id ~equal:String.equal)
        then Some C.name
        else None)
    in
    let count = min 3 (List.length not_in_play) in
    let%bind.Botc_exec () = Botc_exec.wake d in
    let%bind.Botc_exec bluffs =
      Botc_exec.narrator_pick "demon bluffs" not_in_play ~pick_count:count
    in
    let%bind.Botc_exec () =
      Botc_exec.tell d ("Bluffs: " ^ String.concat bluffs ~sep:", ")
    in
    Botc_exec.sleep d
;;

let run_order order night =
  Botc_exec.iter order ~f:(fun (module C : Character_intf.S) ->
    let%bind.Botc_exec state = Botc_exec.get_state in
    match Game_state.find_character_id state C.id with
    | None -> Botc_exec.return ()
    | Some pid ->
      (match C.night_action ~player_id:pid ~night with
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
