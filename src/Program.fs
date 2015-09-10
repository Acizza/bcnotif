module Program

open System
open System.Net

[<Literal>]
let URL = "http://broadcastify.com/listen/top"

let downloadFeedList() = (new WebClient()).DownloadString URL

//let fetchFeeds =

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
