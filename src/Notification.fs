module Notification

open Feed

type Icon =
    | Info
    | Error

#if LINUX
open Notifications

let create icon title body =
    let notif = new Notification()

    notif.IconName <-
        match icon with
        | Info -> "emblem-sound"
        | Error -> "dialog-error"

    notif.Summary <- title
    notif.Body <- body
    notif.Show()
#else
open System.Drawing
open System.Windows.Forms

let create icon title body =
    use notif = new NotifyIcon()

    let sIcon, tIcon =
        match icon with
        | Info  -> SystemIcons.Information, ToolTipIcon.Info
        | Error -> SystemIcons.Error, ToolTipIcon.Error

    notif.Icon <- sIcon
    notif.Visible <- true
    notif.ShowBalloonTip(5000, title, body, tIcon)
#endif

let createFeedUpdate curIdx maxIdx =
    create
        Info
        (sprintf "Broadcastify Listener Update (%d of %d)" curIdx maxIdx)

let createError = create Error "Broadcastify Update Error"