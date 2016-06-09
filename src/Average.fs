module Average

open FSharp.Data
open System
open System.IO
open Util

type Average = {
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
        avg.Moving
        |> List.setFront newMovingData maxMovingSize

    avg.Hourly.[hour] <- moving |> List.averageBy float |> int
    {avg with Moving = moving}

let saveToFile path avgs =
    let avgArr = avgs |> Map.toArray
    let head   = Array.head avgArr |> snd

    avgArr
    |> Array.filter (fun (_, a) -> a.Hourly.Length = head.Hourly.Length)
    |> Array.map (fun (name : string, avg) ->
        let hourly = avg.Hourly |> Array.map string
        sprintf "\"%s\",%s"
            <| name.Trim()
            <| String.concat "," hourly
    )
    |> fun str -> File.WriteAllLines(path, str)

let loadFromFile path =
    match Util.touchFile path with
    | 0L -> Map.empty
    | _  ->
        CsvFile.Load(path).Rows
        |> Seq.fold (fun m x ->
            let avg =
                x.Columns.[1..]
                |> Array.map int
                |> createWithHourly
            Map.add x.[0] avg m
        ) Map.empty