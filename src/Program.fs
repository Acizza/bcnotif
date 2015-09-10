module Program

open System
open System.Net

[<Literal>]
let URL = "http://broadcastify.com/listen/top"

let downloadFeedList (url:string) = (new WebClient()).DownloadString url
let fetchFeeds = downloadFeedList >> Feed.createAllFromString

let processFeeds minListeners (feeds:Feed.Feed array) =
    feeds
    |> Array.filter (fun x -> x.Listeners >= minListeners)
    |> Notification.createFromFeed

[<EntryPoint>]
let main args =
    let (|Double|) str =
        match Double.TryParse str with
        | true,x -> Some x
        | false,_ -> None

    match args |> Array.toList with
    | (Double minListeners)::(Double updateTime)::_ ->
        ()
    | _ ->
        printfn "Usage: <minimum listeners> <update time in minutes>"

    0