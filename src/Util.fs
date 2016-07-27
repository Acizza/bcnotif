module Util

open System
open System.IO

module Convert =
    let tryParse f input =
        match f input with
        | (true, x)  -> Some x
        | (false, _) -> None

module Option =
    /// Flipped version of defaultArg
    let defaultArg x y = defaultArg y x

module List =
    /// Adds the specified value to the front of a list and trims the tail if it's longer than the specified size
    let setFront value maxSize = function
        | xs when List.length xs < maxSize -> value :: xs
        | xs -> value :: xs.[..xs.Length - 2]

/// Creates the file specified if it doesn't exist, returns the length otherwise
let touchFile path =
    let info = FileInfo path

    match info.Exists with
    | true -> info.Length
    | false ->
        use f = File.Create(path)
        0L

/// Returns a path relative to the location of the program's executable
let localPath path =
    System.IO.Path.Combine(
        AppDomain.CurrentDomain.BaseDirectory,
        path)