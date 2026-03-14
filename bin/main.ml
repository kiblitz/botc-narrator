open! Core
open Botc_narrator_lib

let make_console_interp (state : Game_state.t) : (module Narrator_monad.INTERP) =
  let pname id =
    match Map.find state.Game_state.players id with
    | Some p -> p.Player.name
    | None   -> Printf.sprintf "<%d>" (Player_id.to_int id)
  in
  (module struct
    let wake  id = Printf.printf "[wake]  %s\n%!" (pname id)
    let sleep id = Printf.printf "[sleep] %s\n%!" (pname id)

    let show_info id info =
      Printf.printf "[info -> %-10s] %s\n%!" (pname id)
        (match info with
         | Narrator_monad.Two_players_one_char (p1, p2, c) ->
           Printf.sprintf "%s and %s — one is the %s" (pname p1) (pname p2) c
         | Narrator_monad.No_outsiders         -> "no outsiders"
         | Narrator_monad.Number n             -> Int.to_string n
         | Narrator_monad.Yes_or_no b          -> if b then "yes" else "no"
         | Narrator_monad.Character_revealed c -> c
         | Narrator_monad.Grimoire entries ->
           List.map entries ~f:(fun (pid, c) ->
             Printf.sprintf "%s=%s" (pname pid) c)
           |> String.concat ~sep:", "
         | Narrator_monad.Evil_players ids ->
           "evil: " ^ (List.map ids ~f:pname |> String.concat ~sep:", ")
         | Narrator_monad.Demon_bluffs cs ->
           "bluffs: " ^ String.concat ~sep:", " cs)

    let player_points     _id cs = List.hd_exn cs
    let player_points_two _id cs =
      match cs with p1 :: p2 :: _ -> (p1, p2) | _ -> failwith "need ≥2 candidates"
    let narrator_pick = function [] -> failwith "narrator_pick: empty" | x :: _ -> x
    let log s = Printf.printf "[log] %s\n%!" s
  end)

let () =
  let mk i name char =
    Player.create ~id:(Player_id.of_int i) ~name ~character:char
  in
  let players = [
    mk 0 "Alice" (module Characters.Imp           : Character_intf.S);
    mk 1 "Bob"   (module Characters.Poisoner       : Character_intf.S);
    mk 2 "Carol" (module Characters.Empath         : Character_intf.S);
    mk 3 "Dave"  (module Characters.Washerwoman    : Character_intf.S);
    mk 4 "Eve"   (module Characters.Fortune_teller : Character_intf.S);
    mk 5 "Frank" (module Characters.Chef           : Character_intf.S);
  ] in
  let seat_order = List.map players ~f:(fun p -> p.Player.id) in
  let state =
    Game_state.create seat_order players
    |> fun s -> { s with Game_state.phase = Game_state.Night { number = 1 } }
  in
  let interp = make_console_interp state in
  Printf.printf "=== Night 1 ===\n%!";
  let ((), _final) = Narrator_monad.run interp state (Trouble_brewing.run_night ()) in
  ()
