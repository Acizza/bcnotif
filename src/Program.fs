module Main

open Config
open Config.Args
open Feed
open Feed.Average
open System
open System.Threading
open Util

module Path =
    let averages = Util.localPath "averages.csv"
    let config   = Util.localPath "Config.yaml"

let update threshold sortOrder avgs (config : Config) =
    let hour  = DateTime.UtcNow.Hour
    let feeds =
        Feed.createFromURL "http://www.broadcastify.com/listen/top"
        |> Array.map (fun f ->
            let avg =
                avgs
                |> Map.tryFind f.Id
                |> Option.defaultArg f.AvgListeners
                |> Average.update hour 5 f.Listeners
            {f with AvgListeners = avg}
        )

    feeds
    |> Feed.filter config hour threshold
    |> Feed.displayAll sortOrder

    let newAvgs =
        Array.fold (fun m f -> Map.add f.Id f.AvgListeners m)
            avgs
            feeds

    Average.saveToFile Path.averages newAvgs
    newAvgs

let start threshold (updateTime : TimeSpan) sortOrder =
    let config = Config()

    let rec loop avgs =
        let newAvgs =
            try
                config.Load Path.config
                update threshold sortOrder avgs config
            with
            | ex ->
                eprintfn "%A" ex
                ex.ToString() |> Notification.createError
                avgs

        Thread.Sleep (int updateTime.TotalMilliseconds)
        loop newAvgs

    Average.loadFromFile Path.averages
    |> loop

[<EntryPoint>]
let main args =
    match Args.tryParse args with
    | Success (parser, results) ->
        let threshold  = (results |> get <@ Threshold @> 30.) / 100. + 1.
        let updateTime = results  |> get <@ UpdateTime @> 6.
        let sortOrder  = results  |> get <@ Sort @> Descending

        let (|Minutes|) = TimeSpan.FromMinutes

        match updateTime with
        | t when t < 5. -> parser.PrintUsage() |> eprintfn "%s"
        | Minutes time  -> start threshold time sortOrder
    | Failure msg -> eprintfn "%s" msg

    0