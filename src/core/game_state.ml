open! Core

type phase =
  | Setup
  | Day   of { number : int }
  | Night of { number : int }
[@@deriving sexp]

type t =
  { (** Players in seating order — order matters for Chef/Empath adjacency. *)
    seat_order     : Player_id.t list
  ; players        : Player.t Player_id.Map.t
  ; phase          : phase
  ; poisoned       : Player_id.t option
  ; monk_protected : Player_id.t option
  (** Players killed during the current night (announced at dawn). *)
  ; night_deaths   : Player_id.t list
  (** Player executed yesterday, for Undertaker. *)
  ; last_execution : Player_id.t option
  ; imp_starpass   : bool
  }

let create seat_order players_list =
  let players =
    List.fold players_list ~init:Player_id.Map.empty
      ~f:(fun m p -> Map.set m ~key:p.Player.id ~data:p)
  in
  { seat_order
  ; players
  ; phase          = Setup
  ; poisoned       = None
  ; monk_protected = None
  ; night_deaths   = []
  ; last_execution = None
  ; imp_starpass   = false
  }

let seated_players t =
  List.filter_map t.seat_order ~f:(Map.find t.players)

let alive_players t =
  List.filter (seated_players t) ~f:(fun p -> p.Player.alive)

let alive_ids t =
  List.map (alive_players t) ~f:(fun p -> p.Player.id)

let find_character_id t char_id =
  List.find (seated_players t)
    ~f:(fun p -> String.equal (Player.character_id p) char_id)

let is_poisoned t id =
  Option.equal Player_id.equal t.poisoned (Some id)

let kill t id =
  let players =
    Map.update t.players id ~f:(function
      | None   -> failwith "kill: unknown player"
      | Some p -> { p with Player.alive = false; has_ghost_vote = true })
  in
  { t with players; night_deaths = id :: t.night_deaths }

let set_poisoned       t id = { t with poisoned       = Some id }
let clear_poisoned     t    = { t with poisoned        = None   }
let set_monk_protected t id = { t with monk_protected  = Some id }
let clear_monk_protected t  = { t with monk_protected  = None   }

let begin_night t n =
  { t with
    phase          = Night { number = n }
  ; night_deaths   = []
  ; monk_protected = None
  ; imp_starpass   = false
  }

let begin_day t n = { t with phase = Day { number = n } }

let character_ids_in_play t =
  List.map (seated_players t) ~f:Player.character_id
  |> List.dedup_and_sort ~compare:String.compare

let good_char_names_not_in_play t (all_chars : Character_intf.t list) =
  let in_play = character_ids_in_play t in
  List.filter_map all_chars ~f:(fun c ->
    if Character_intf.is_good c
    && not (List.mem in_play (Character_intf.id c) ~equal:String.equal)
    then Some (Character_intf.name c)
    else None)
