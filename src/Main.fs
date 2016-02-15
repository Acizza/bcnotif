module Main

open System
open System.Net
open System.IO
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
        match Map.tryFind f.Name avg with
        | Some x -> float f.Listeners >= x * threshold
        | None   -> false

    let feeds' =
        feeds
        |> Array.filter (fun x -> isPastAverage x || Option.isSome x.Info)
        #if WINDOWS // Windows displays notifications as FILO; Linux displays them as FIFO
        |> Array.rev
        #endif

    let create i x =
        let idx =
            #if WINDOWS // Reverse index on Windows to accommodate for the FILO method
            feeds'.Length-i
            #else
            i+1
            #endif

        Notification.createUpdate
            idx
            (Array.length feeds')
            (x.ToString())

    Array.iteri create feeds'
    allFeeds

/// Starts the processing loop to fetch and display feeds
let runLoop threshold (updateTime:TimeSpan) =
    let rec run timesSinceSave = function
        | xs when timesSinceSave >= 5 ->
            PreviousData.save PreviousData.filePath xs
            run 0 xs
        | xs when Array.length xs >= 5 ->
            Array.tail xs |> run timesSinceSave
        | prevFeeds ->
                let feeds =
                    getFeeds ()
                    |> processFeeds threshold prevFeeds

                Thread.Sleep (updateTime.TotalMilliseconds |> int)
                run (timesSinceSave+1) feeds

    let pastListeners =
        try
            PreviousData.getOrCreate PreviousData.filePath [||]
        with
        | ex ->
            Notification.createError ex.Message
            [||]

    run 0 pastListeners

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
        runLoop 30. (TimeSpan.FromMinutes 6.)
    0