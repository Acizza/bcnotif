module Average

open System
open System.Collections.Generic
open System.IO
open FSharp.Data

type T = {
    Moving : int list
    Hourly : int array
}

let create hourlySize = {
    Moving = []
    Hourly = Array.zeroCreate hourlySize
}

let createWithHourly source = {
    Moving = []
    Hourly = source
}

let update hour maxMovingSize newMovingData avg =
    let hourly = avg.Hourly.[hour]
    let moving =
        match avg.Moving with
        | xs when xs.Length < maxMovingSize -> newMovingData :: xs
        | xs -> newMovingData :: xs.[..xs.Length - 2]
        | _  -> if hourly = 0 then [newMovingData] else [hourly]

    avg.Hourly.[hour] <- moving |> List.averageBy float |> int
    {avg with Moving = moving}

let filePath =
    AppDomain.CurrentDomain.BaseDirectory
    + "prevlisteners.csv"

let saveHourly (avgs : (string * T) seq) =
    let head = Seq.head avgs |> snd
    let data =
        avgs
        |> Seq.filter (fun (_, avg) -> avg.Hourly.Length = head.Hourly.Length)
        |> Seq.map (fun (name, avg) ->
            let listeners = Array.map string avg.Hourly
            sprintf "\"%s\",%s"
                <| name.Trim()
                <| String.concat "," listeners
        )
        |> Array.ofSeq

    File.WriteAllLines(filePath, data)

/// Creates the file specified if it doesn't exist, returns the length otherwise
let private touchFile path =
    let info = FileInfo path

    match info.Exists with
    | true -> info.Length
    | false ->
        use f = File.Create(path)
        0L

/// Loads the CSV file containing hourly averages
let loadHourly () =
    let length = touchFile filePath
    match length with
    | 0L -> seq []
    | _  ->
        CsvFile.Load(filePath, hasHeaders=false).Rows
        |> Seq.map (fun xs ->
            let columns =
                xs.Columns.[1..]
                |> Array.map int

            (xs.[0], columns)
        )