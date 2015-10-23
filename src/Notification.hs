module Notification
    ( create
    , createError
    , createFeedUpdate
    ) where

import Libnotify

type Title = String
type Body = String
type Icon = String

create :: Icon -> Title -> Body -> IO Notification
create icon' title body' =
    display $
        summary title
     <> body body'
     <> icon icon'
     <> timeout Default

createError :: Body -> IO Notification
createError = create "dialog-error" "Broadcastify Update Error"

createFeedUpdate :: Title -> Body -> IO Notification
createFeedUpdate = create "emblem-sound"
