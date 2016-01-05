module Main

open System
open System.Net
open System.Threading
open Feed

let downloadURL (url:string) =
    use c = new WebClient()
    c.DownloadString url
    |> fun x -> x.Trim().Replace("\n", " ")

let getFeeds () =
    try
        downloadURL "http://www.broadcastify.com/listen/top"
        |> Feed.createFromString
    with
    | ex ->
        Notification.createError ex.Message
        [||]

/// Filters feeds by a threshold and displays them as notifications
let processFeeds threshold prevFeeds feeds =
    let allFeeds = Array.append prevFeeds [|feeds|]
    let avg      = Feed.getAverageListeners allFeeds

    let isPastAverage f =
        match Map.tryFind f.name avg with
        | Some x -> float f.listeners >= x * threshold
        | None   -> false

    let feeds' =
        feeds
        |> Array.filter (fun x -> isPastAverage x || Option.isSome x.info)
        #if WINDOWS // Windows displays notifications as FILO; Linux displays them as FIFO
        |> Array.rev
        #endif

    Array.iteri (fun i x ->
        Notification.createUpdate
            (i+1)
            (Array.length feeds')
            (x.ToString())
    ) feeds'
    allFeeds

/// Starts the processing loop to fetch and display feeds
let runLoop threshold (updateTime:TimeSpan) =
    let rec run = function
        | xs when Array.length xs > 5 ->
            Array.tail xs |> run
        | prevFeeds ->
                let feeds =
                    getFeeds ()
                    |> processFeeds threshold prevFeeds

                Thread.Sleep (updateTime.TotalMilliseconds |> int)
                run feeds

    run [||]

[<EntryPoint>]
let main args =
    let tryParse f input =
        match f input with
        | true,x -> Some x
        | false,_ -> None

    let (|Minutes|_|) =
        tryParse Double.TryParse
        >> Option.map TimeSpan.FromMinutes
        
    /// Returns a percentage as a multiplier
    let (|Threshold|_|) =
        tryParse Double.TryParse
        >> Option.map (fun x -> x / 100. + 1.)

    match args with
    | [|_; Minutes time|] when time.TotalMinutes < 5. ->
        Console.WriteLine "Update time must be >= 5 minutes"
    | [|Threshold threshold; Minutes updateTime|] ->
        runLoop threshold updateTime
    | _ ->
        Console.WriteLine "Usage: <percentage jump to display feed> <update time in minutes>"
        runLoop 35. (TimeSpan.FromMinutes 6.)
    0
