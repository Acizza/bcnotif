module Config

open Argu
open FSharp.Configuration
open Util

module Args =
    open System

    type SortOrder =
        | Ascending
        | Descending

    type Arguments =
        | [<AltCommandLine("-p")>] Threshold  of percentage:float
        | [<AltCommandLine("-t")>] UpdateTime of minutes:float
        | [<AltCommandLine("-s")>] Sort       of SortOrder
        with
            interface IArgParserTemplate with
                member s.Usage =
                    match s with
                    | Threshold  _ -> "specify the percentage jump required to show a feed"
                    | UpdateTime _ -> "specify the minutes between updates (must be >= 5)"
                    | Sort       _ -> "specify the order feeds will be displayed in"

    let tryParse args =
        let parser = ArgumentParser.Create<Arguments>()
        try
            Success (parser, parser.Parse args)
        with
        | :? ArguParseException ->
            Failure (parser.PrintUsage())
        | ex -> Failure ex.Message

    let get (value : Quotations.Expr<('Field -> _)>) def (results : ParseResults<_>) =
        results.GetResult(value, defaultValue = def)

type Config = YamlConfig<"Config.yaml">
let config  = Config()