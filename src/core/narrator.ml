open! Core

let minion_info () =
  let%bind.Botc_exec state = Botc_exec.get_state () in
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
  let%bind.Botc_exec state = Botc_exec.get_state () in
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

(** Combine a list of read-only actions into a single computation using [Both]
    nodes so the engine can run them in parallel. Logs parallel markers when
    there are multiple actions. *)
let combine_ro actions =
  let indexed =
    List.mapi actions ~f:(fun i m ->
      let%bind.Botc_exec () = Botc_exec.log [%string "[parallel %{i#Int}]"] in
      m)
  in
  let rec go = function
    | [] -> Botc_exec.return ()
    | [ m ] -> m
    | m :: rest ->
      let%map.Botc_exec () = m
      and () = go rest in
      ()
  in
  go indexed
;;

(** Split a list of tagged actions into a leading batch of [Read_only] actions
    and the remaining tail starting from the first [Read_write] action. *)
let rec take_ro_prefix = function
  | Character_intf.Read_only m :: rest ->
    let batch, remaining = take_ro_prefix rest in
    m :: batch, remaining
  | other -> [], other
;;

(** Run a list of tagged night actions, batching consecutive [Read_only] actions
    in parallel and treating [Read_write] actions as sequential sync points. *)
let rec run_batched = function
  | [] -> Botc_exec.return ()
  | Character_intf.Read_write m :: rest ->
    let%bind.Botc_exec () = m in
    run_batched rest
  | Character_intf.Read_only _ :: _ as actions ->
    let ro_batch, rest = take_ro_prefix actions in
    let%bind.Botc_exec () = Botc_exec.as_rw (combine_ro ro_batch) in
    run_batched rest
;;

let run_order order night =
  let%bind.Botc_exec state = Botc_exec.get_state () in
  let tagged =
    List.filter_map order ~f:(fun (module C : Character_intf.S) ->
      match Game_state.find_character_id state C.id with
      | None -> None
      | Some pid -> C.night_action ~player_id:pid ~night)
  in
  run_batched tagged
;;

let run_night (module S : Script_intf.S) () =
  let%bind.Botc_exec state = Botc_exec.get_state () in
  match Game_state.phase state with
  | Game_state.Phase.Night { number = 1 } ->
    let%bind.Botc_exec () = minion_info () in
    let%bind.Botc_exec () = demon_info S.all () in
    run_order S.night_one_order 1
  | Game_state.Phase.Night { number = n } -> run_order S.night_subsequent_order n
  | _ -> Botc_exec.return ()
;;
