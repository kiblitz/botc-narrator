open! Core
open Botc_narrator_lib

let p = Player_id.of_string

let mk_players specs =
  List.map specs ~f:(fun (name, (module C : Character_intf.S)) ->
    { Game_state.Player_spec.id = p name
    ; character_id = C.id
    ; character_name = C.name
    ; kind = C.kind C.t
    ; alignment = C.alignment C.t
    })
;;

let mk_state specs =
  let seat_order = List.map specs ~f:(fun (s : Game_state.Player_spec.t) -> s.id) in
  Game_state.create seat_order specs
;;

let night_state specs = mk_state specs |> Game_state.next_phase
let day_state specs = mk_state specs |> Game_state.next_phase |> Game_state.next_phase

let make_test_interp ~state ?(responses = []) ?(silent = false) () =
  let pname id =
    let name = Player_id.to_string id in
    let role = Game_state.character_name state id in
    [%string "%{name}(%{role})"]
  in
  let responses = ref responses in
  (module struct
    let print = if silent then ignore else print_endline
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

    let narrator_pick _prompt xs = List.hd_exn xs
    let log s = print [%string "[log] %{s}"]
  end : Botc_exec.Engine_S)
;;

let run ?(silent = false) ?(responses = []) ~action state =
  let interp = make_test_interp ~state ~responses ~silent () in
  let (), state = Botc_exec.run interp state action in
  state
;;

let night_action action ~night ~player_id =
  Option.value_exn (action ~player_id:(p player_id) ~night)
;;

let day_action action ~player_id = Option.value_exn (action ~player_id:(p player_id))

let print_grimoire state =
  let seated = Game_state.seat_order state in
  let label id =
    let name = Player_id.to_string id in
    let role = Game_state.character_name state id in
    let tags =
      List.filter_opt
        [ (if not (Game_state.is_alive state id) then Some "dead" else None)
        ; (if Game_state.is_poisoned state id then Some "poisoned" else None)
        ]
    in
    match tags with
    | [] -> [%string "%{name}(%{role})"]
    | ts -> [%string "%{name}(%{role}) [%{String.concat ts ~sep:\", \"}]"]
  in
  let labels = List.mapi seated ~f:(fun i id -> i, label id) in
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
  let min_r =
    List.map positions ~f:(fun (_, r, _) -> r)
    |> List.min_elt ~compare:Int.compare
    |> Option.value ~default:0
  in
  let min_c =
    List.map positions ~f:(fun (lbl, _, c) -> c - (String.length lbl / 2))
    |> List.min_elt ~compare:Int.compare
    |> Option.value ~default:0
  in
  let rows = (2 * Float.iround_nearest_exn radius_r) + 2 in
  let cols =
    List.map positions ~f:(fun (lbl, _, c) ->
      c - min_c + (String.length lbl / 2) + String.length lbl)
    |> List.max_elt ~compare:Int.compare
    |> Option.value ~default:0
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
