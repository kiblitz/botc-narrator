open! Core

module type Engine_S = sig
  val wake : Player_id.t -> unit
  val sleep : Player_id.t -> unit
  val tell : Player_id.t -> string -> unit
  val ask : Player_id.t -> string -> Player_id.t list -> Player_id.t
  val narrator_pick : string -> 'a list -> 'a
  val log : string -> unit
end

module type Botc_exec = sig
  (** Abstract free monad type. Build values using the combinators below. *)
  type 'a t

  val return : 'a -> 'a t
  val bind : 'a t -> f:('a -> 'b t) -> 'b t
  val map : 'a t -> f:('a -> 'b) -> 'b t
  val ( >>= ) : 'a t -> ('a -> 'b t) -> 'b t
  val ( >>| ) : 'a t -> ('a -> 'b) -> 'b t

  (** Use [let%bind.Botc_exec] / [let%map.Botc_exec] for monadic syntax.
      Supports [and] for independent effects. *)
  module Let_syntax : sig
    val return : 'a -> 'a t
    val ( >>= ) : 'a t -> ('a -> 'b t) -> 'b t

    module Let_syntax : sig
      val return : 'a -> 'a t
      val bind : 'a t -> f:('a -> 'b t) -> 'b t
      val map : 'a t -> f:('a -> 'b) -> 'b t
      val both : 'a t -> 'b t -> ('a * 'b) t

      module Open_on_rhs : sig end
    end
  end

  val wake : Player_id.t -> unit t
  val sleep : Player_id.t -> unit t
  val tell : Player_id.t -> string -> unit t
  val ask : Player_id.t -> string -> Player_id.t list -> Player_id.t t
  val narrator_pick : string -> 'a list -> 'a t
  val narrator_pick_from : string -> 'a list -> pick_count:int -> 'a list t
  val log : string -> unit t
  val get_state : Game_state.t t
  val set_state : Game_state.t -> unit t
  val modify_state : (Game_state.t -> Game_state.t) -> unit t
  val when_ : bool -> unit t -> unit t
  val iter : 'a list -> f:('a -> unit t) -> unit t

  module type Engine_S = Engine_S

  val run : (module Engine_S) -> Game_state.t -> 'a t -> 'a * Game_state.t
end
