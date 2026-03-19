open! Core

module Phase = struct
  type t =
    | Setup
    | Day of { number : int }
    | Night of { number : int }
  [@@deriving sexp]
end

module Player_spec = struct
  type t =
    { id : Player_id.t
    ; character_id : string
    ; character_name : string
    ; kind : Character0.Kind.t
    ; alignment : Character0.Alignment.t
    }
end

type player =
  { character_id : string
  ; character_name : string
  ; kind : Character0.Kind.t
  ; alignment : Character0.Alignment.t
  ; alive : bool
  ; has_ghost_vote : bool
  ; poisoned : bool
  ; monk_protected : bool
  ; used_day_ability : bool
  }
[@@deriving sexp_of]

type t =
  { seat_order : Player_id.t list
  ; players : player Player_id.Map.t
  ; phase : Phase.t
  ; night_deaths : Player_id.t list
  ; last_execution : Player_id.t option
  }
[@@deriving fields ~getters, sexp_of]

let create seat_order specs =
  let players =
    List.map specs ~f:(fun (s : Player_spec.t) ->
      ( s.id
      , { character_id = s.character_id
        ; character_name = s.character_name
        ; kind = s.kind
        ; alignment = s.alignment
        ; alive = true
        ; has_ghost_vote = false
        ; poisoned = false
        ; monk_protected = false
        ; used_day_ability = false
        } ))
    |> Player_id.Map.of_alist_exn
  in
  { seat_order; players; phase = Phase.Setup; night_deaths = []; last_execution = None }
;;

let get t id = Map.find_exn t.players id

let update_player t id ~f =
  { t with
    players =
      Map.update t.players id ~f:(function
        | None -> failwith "update_player: unknown player"
        | Some p -> f p)
  }
;;

(** Player queries *)

let is_alive t id = (get t id).alive
let kind t id = (get t id).kind
let alignment t id = (get t id).alignment
let character_id t id = (get t id).character_id
let character_name t id = (get t id).character_name

let is_evil t id =
  Character0.Alignment.equal (get t id).alignment Character0.Alignment.Evil
;;

let is_good t id =
  Character0.Alignment.equal (get t id).alignment Character0.Alignment.Good
;;

let alive_ids t = List.filter t.seat_order ~f:(fun id -> (get t id).alive)

let find_character_id t char_id =
  List.find t.seat_order ~f:(fun pid -> String.equal (get t pid).character_id char_id)
;;

let is_poisoned t id =
  match Map.find t.players id with
  | Some p -> p.poisoned
  | None -> false
;;

let monk_protected t id =
  match Map.find t.players id with
  | Some p -> p.monk_protected
  | None -> false
;;

let has_used_day_ability t id =
  match Map.find t.players id with
  | Some p -> p.used_day_ability
  | None -> false
;;

(** Mutations *)

let kill t id =
  let killed = get t id in
  let t =
    update_player t id ~f:(fun p -> { p with alive = false; has_ghost_vote = true })
  in
  let t = { t with night_deaths = id :: t.night_deaths } in
  if not (Character0.Kind.equal killed.kind Character0.Kind.Demon)
  then t
  else (
    match
      List.find (alive_ids t) ~f:(fun pid ->
        String.equal (get t pid).character_id "scarlet_woman")
    with
    | None -> t
    | Some sw_id ->
      if List.length (alive_ids t) < 5
      then t
      else
        update_player t sw_id ~f:(fun p ->
          { p with
            character_id = killed.character_id
          ; character_name = killed.character_name
          ; kind = killed.kind
          ; alignment = killed.alignment
          }))
;;

let clear_poisoned t =
  { t with players = Map.map t.players ~f:(fun p -> { p with poisoned = false }) }
;;

let set_poisoned t id =
  let t = clear_poisoned t in
  update_player t id ~f:(fun p -> { p with poisoned = true })
;;

let clear_monk_protected t =
  { t with players = Map.map t.players ~f:(fun p -> { p with monk_protected = false }) }
;;

let set_monk_protected t id =
  let t = clear_monk_protected t in
  update_player t id ~f:(fun p -> { p with monk_protected = true })
;;

let use_day_ability t id =
  update_player t id ~f:(fun p -> { p with used_day_ability = true })
;;

let transform_into t id ~character_id ~character_name ~kind ~alignment =
  update_player t id ~f:(fun p ->
    { p with character_id; character_name; kind; alignment })
;;

let next_phase t =
  match t.phase with
  | Phase.Setup ->
    { (clear_monk_protected t) with
      phase = Phase.Night { number = 1 }
    ; night_deaths = []
    }
  | Phase.Night { number } -> { t with phase = Phase.Day { number } }
  | Phase.Day { number } ->
    { (clear_monk_protected t) with
      phase = Phase.Night { number = number + 1 }
    ; night_deaths = []
    }
;;

let character_ids_in_play t =
  List.map t.seat_order ~f:(fun id -> (get t id).character_id)
  |> List.dedup_and_sort ~compare:String.compare
;;
