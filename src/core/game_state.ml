open! Core

module Phase = struct
  type t =
    | Setup
    | Day of { number : int }
    | Night of { number : int }
  [@@deriving sexp]
end

type t =
  { (* Seating order matters for Chef/Empath adjacency. *)
    seat_order : Player_id.t list
  ; players : Player.t Player_id.Map.t
  ; phase : Phase.t
  ; poisoned : Player_id.t option
  ; monk_protected : Player_id.t option
  ; night_deaths : Player_id.t list (* Killed during the current night. *)
  ; last_execution : Player_id.t option (* Executed yesterday, for Undertaker. *)
  ; used_day_abilities : Player_id.Set.t (* Once-per-game day abilities, e.g. Slayer. *)
  }
[@@deriving fields ~getters, sexp_of]

let create seat_order players_list =
  let players =
    List.fold players_list ~init:Player_id.Map.empty ~f:(fun m p ->
      Map.set m ~key:(Player.id p) ~data:p)
  in
  { seat_order
  ; players
  ; phase = Phase.Setup
  ; poisoned = None
  ; monk_protected = None
  ; night_deaths = []
  ; last_execution = None
  ; used_day_abilities = Player_id.Set.empty
  }
;;

let seated_players t = List.filter_map (seat_order t) ~f:(Map.find (players t))
let alive_players t = List.filter (seated_players t) ~f:Player.alive
let alive_ids t = List.map (alive_players t) ~f:Player.id

let find_character_id t char_id =
  List.find (seated_players t) ~f:(fun p -> String.equal (Player.character_id p) char_id)
;;

let is_poisoned t id = Option.equal Player_id.equal (poisoned t) (Some id)

let kill t id =
  let players' =
    Map.update (players t) id ~f:(function
      | None -> failwith "kill: unknown player"
      | Some p -> { p with Player.alive = false; has_ghost_vote = true })
  in
  let t' = { t with players = players'; night_deaths = id :: t.night_deaths } in
  (* Scarlet Woman trigger: if the Demon died with 5+ players still alive,
     the Scarlet Woman becomes the new Demon. *)
  let killed = Map.find_exn (players t) id in
  if not (Char_display.is_demon (Player.character killed))
  then t'
  else (
    match
      List.find (alive_players t') ~f:(fun p ->
        String.equal (Player.character_id p) "scarlet_woman")
    with
    | None -> t'
    | Some sw ->
      if List.length (alive_players t') < 5
      then t'
      else (
        let new_players =
          Map.update (players t') (Player.id sw) ~f:(function
            | None -> failwith "scarlet_woman trigger: missing player"
            | Some p -> { p with Player.character = Player.character killed })
        in
        { t' with players = new_players }))
;;

let set_players t players = { t with players }
let set_poisoned t id = { t with poisoned = Some id }
let clear_poisoned t = { t with poisoned = None }
let set_monk_protected t id = { t with monk_protected = Some id }
let clear_monk_protected t = { t with monk_protected = None }
let use_day_ability t id = { t with used_day_abilities = Set.add t.used_day_abilities id }
let has_used_day_ability t id = Set.mem t.used_day_abilities id

let next_phase t =
  match t.phase with
  | Phase.Setup ->
    { t with phase = Phase.Night { number = 1 }; night_deaths = []; monk_protected = None }
  | Phase.Night { number } -> { t with phase = Phase.Day { number } }
  | Phase.Day { number } ->
    { t with
      phase = Phase.Night { number = number + 1 }
    ; night_deaths = []
    ; monk_protected = None
    }
;;

let character_ids_in_play t =
  List.map (seated_players t) ~f:Player.character_id
  |> List.dedup_and_sort ~compare:String.compare
;;

let good_chars_not_in_play t (all_chars : Char_display.t list) =
  let in_play = character_ids_in_play t in
  List.filter all_chars ~f:(fun c ->
    Char_display.is_good c
    && not (List.mem in_play (Char_display.id c) ~equal:String.equal))
;;
