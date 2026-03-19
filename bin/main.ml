open! Core
open Botc_narrator_lib

let make_console_interp (state : Game_state.t) : (module Botc_exec.Interp_S) =
  let pname id =
    match Map.find (Game_state.players state) id with
    | Some p -> Player.name p
    | None -> Player_id.to_string id
  in
  (module struct
    let wake id = printf "[wake]  %s\n%!" (pname id)
    let sleep id = printf "[sleep] %s\n%!" (pname id)
    let tell id msg = printf "[tell -> %-10s] %s\n%!" (pname id) msg
    let ask _id _question cs = List.hd_exn cs

    let narrator_pick = function
      | [] -> failwith "narrator_pick: empty"
      | x :: _ -> x
    ;;

    let log s = printf "[log] %s\n%!" s
  end)
;;

let () =
  let mk name char = Player.create ~id:(Player_id.of_string name) ~character:char in
  let players =
    [ mk "Alice" (Characters.Imp.to_display Characters.Imp.t)
    ; mk "Bob" (Characters.Poisoner.to_display Characters.Poisoner.t)
    ; mk "Carol" (Characters.Empath.to_display Characters.Empath.t)
    ; mk "Dave" (Characters.Washerwoman.to_display Characters.Washerwoman.t)
    ; mk "Eve" (Characters.Fortune_teller.to_display Characters.Fortune_teller.t)
    ; mk "Frank" (Characters.Chef.to_display Characters.Chef.t)
    ]
  in
  let seat_order = List.map players ~f:Player.id in
  let state =
    Game_state.create seat_order players
    |> fun s -> { s with Game_state.phase = Game_state.Phase.Night { number = 1 } }
  in
  let interp = make_console_interp state in
  printf "=== Night 1 ===\n%!";
  let (), _final =
    Botc_exec.run interp state (Narrator.run_night (module Trouble_brewing) ())
  in
  ()
;;
