open! Core
open Botc_narrator_lib

let p = Player_id.of_string

let mk_players specs =
  List.map specs ~f:(fun (name, (module C : Character_intf.S)) ->
    Player.create ~id:(p name) ~character:(C.to_display C.t))
;;

let mk_state players =
  let seat_order = List.map players ~f:Player.id in
  Game_state.create seat_order players
;;

let night_state players = mk_state players |> Game_state.next_phase
let day_state players = mk_state players |> Game_state.next_phase |> Game_state.next_phase

let make_test_interp ~players ?(responses = []) ?(silent = false) () =
  let pname id =
    match List.find players ~f:(fun p -> Player_id.equal (Player.id p) id) with
    | Some p ->
      let role = Char_display.name (Player.character p) in
      [%string "%{Player.name p}(%{role})"]
    | None -> Player_id.to_string id
  in
  let responses = ref responses in
  (module struct
    let print = if silent then Fun.ignore else print_endline

    let wake id = print [%string "narrator->%{pname id}: wake"]
    let sleep id = print [%string "narrator->%{pname id}: sleep"]
    let tell id msg = print [%string "narrator->%{pname id}: %{msg}"]

    let ask id question cs =
      print [%string "narrator->%{pname id}: %{question}"];
      let choice =
        match !responses with
        | r :: rest ->
          responses := rest;
          r
        | [] -> List.hd_exn cs
      in
      print [%string "%{pname id}->narrator: %{pname choice}"];
      choice
    ;;

    let narrator_pick xs = List.hd_exn xs
    let log s = print [%string "[log] %{s}"]
  end : Botc_exec.Interp_S)
;;

let run ~players ?(silent = false) ?(responses = []) ~action state =
  let interp = make_test_interp ~players ~responses ~silent () in
  let (), state = Botc_exec.run interp state action in
  state
;;

let night_action action ~night ~player_id =
  Option.value_exn (action ~player_id:(p player_id) ~night)
;;

let day_action action ~player_id = Option.value_exn (action ~player_id:(p player_id))

let print_grimoire state =
  let seated = Game_state.seated_players state in
  let label player =
    let name = Player.name player in
    let role = Char_display.name (Player.character player) in
    let tags =
      List.filter_opt
        [ (if not (Player.alive player) then Some "dead" else None)
        ; (if Game_state.is_poisoned state (Player.id player)
           then Some "poisoned"
           else None)
        ]
    in
    match tags with
    | [] -> [%string "%{name}(%{role})"]
    | ts -> [%string "%{name}(%{role}) [%{String.concat ts ~sep:\", \"}]"]
  in
  let labels = List.mapi seated ~f:(fun i p -> i, label p) in
  let n = List.length labels in
  let pi = Float.pi in
  let radius_r = 6.0 in
  let radius_c = 38.0 in
  let positions =
    List.map labels ~f:(fun (i, lbl) ->
      let angle = (-.pi /. 2.0) +. (2.0 *. pi *. Float.of_int i /. Float.of_int n) in
      let r = Float.iround_nearest_exn (radius_r *. Float.sin angle) in
      let c = Float.iround_nearest_exn (radius_c *. Float.cos angle) in
      lbl, r, c)
  in
  let min_r = List.fold positions ~init:0 ~f:(fun acc (_, r, _) -> min acc r) in
  let min_c =
    List.fold positions ~init:0 ~f:(fun acc (lbl, _, c) ->
      min acc (c - (String.length lbl / 2)))
  in
  let rows = (2 * Float.iround_nearest_exn radius_r) + 2 in
  let cols =
    List.fold positions ~init:0 ~f:(fun acc (lbl, _, c) ->
      max acc (c - min_c + (String.length lbl / 2) + String.length lbl))
  in
  let grid = Array.init rows ~f:(fun _ -> Bytes.make cols ' ') in
  List.iter positions ~f:(fun (lbl, r, c) ->
    let gr = r - min_r in
    let gc = c - min_c - (String.length lbl / 2) in
    let gc = max 0 gc in
    String.iteri lbl ~f:(fun j ch ->
      if gc + j < cols then Bytes.set grid.(gr) (gc + j) ch));
  let non_empty =
    Array.filter_mapi grid ~f:(fun i row ->
      let s = Bytes.to_string row |> String.rstrip in
      if not (String.is_empty s) then Some (i, s) else None)
  in
  match Array.length non_empty with
  | 0 -> ()
  | _ ->
    let first_i = fst non_empty.(0) in
    let last_i = fst non_empty.(Array.length non_empty - 1) in
    for i = first_i to last_i do
      print_endline (String.rstrip (Bytes.to_string grid.(i)))
    done
;;
