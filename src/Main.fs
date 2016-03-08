module Main

open System
open System.Threading
open Feed

let showNotifications feeds avgs =
    let feeds' =
        feeds
        |> Array.filter (fun f -> Averages.isPastAverage f avgs || Option.isSome f.Info)
        #if WINDOWS // Windows displays notifications as FILO; Linux displays them as FIFO
        |> Array.rev
        #endif

    let create i f =
        let idx =
            #if WINDOWS // Reverse index on Windows to accommodate for the FILO method
            feeds'.Length-i
            #else
            i+1
            #endif

        Feed.createNotif f (i+1) feeds'.Length

    feeds' |> Array.iteri create

/// Filters feeds by a threshold and displays them as notifications
let processFeeds threshold avgs =
    let feeds = Feed.createFromURL "http://www.broadcastify.com/listen/top"

    let avgs =
        Array.fold (fun map f ->
            let value =
                match Map.tryFind f.Name map with
                | Some xs when List.length xs < 5 -> f.Listeners :: xs
                | Some xs -> f.Listeners :: xs.[..List.length xs - 2]
                | _       -> [f.Listeners]

            Map.add f.Name value map
        ) avgs feeds

    showNotifications feeds avgs
    avgs

let try' def f x =
    try
        f x
    with
    | ex ->
        Console.WriteLine ex.Message
        Notification.createError ex.Message
        def

/// Starts the processing loop to fetch and display feeds
let runLoop threshold (updateTime:TimeSpan) =
    let rec run i = function
        | avgs when i >= 5 ->
            try' () Averages.save avgs
            run 0 avgs
        | avgs ->
                let avgs' = try' avgs (processFeeds threshold) avgs

                Thread.Sleep (updateTime.TotalMilliseconds |> int)
                run (i+1) avgs'

    let avgs = try' Map.empty Averages.load ()
    run 0 avgs

[<EntryPoint>]
let main args =
    let tryParse f input =
        match f input with
        | true,x -> Some x
        | false,_ -> None

    let (|Minutes|_|) =
        tryParse Double.TryParse
        >> Option.map TimeSpan.FromMinutes

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
        runLoop 30. (TimeSpan.FromMinutes 6.)
    0