module Notification

#if LINUX
open Notifications
#else
open System.Drawing
open System.Windows.Forms
#endif

type Icon =
    | Info
    | Error

let private create icon title body =
    #if LINUX
        let n = new Notification()

        n.IconName <-
            match icon with
            | Info  -> "emblem-sound"
            | Error -> "dialog-error"

        n.Summary <- title
        n.Body <- body
        n.Show() |> ignore
    #else
        use n = new NotifyIcon()

        let sysIcon, toolIcon =
            match icon with
            | Info  -> SystemIcons.Information, ToolTipIcon.Info
            | Error -> SystemIcons.Error, ToolTipIcon.Error

        n.Icon <- sysIcon
        n.Visible <- true
        n.ShowBalloonTip(5000, title, body, toolIcon) |> ignore
    #endif

let createUpdate curIdx maxIdx =
    create
        Info
        (sprintf "Broadcastify Listener Update (%d of %d)" curIdx maxIdx)

let createError = create Error "Broadcastify Update Error"
