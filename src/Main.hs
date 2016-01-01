{-# LANGUAGE ViewPatterns #-}

module Main where

import Feed
import Control.Concurrent (threadDelay)
import Control.Monad (void)
import Data.Maybe (isJust)
import Network.HTTP (simpleHTTP, getRequest, getResponseBody)
import Text.Read (readMaybe)
import Text.Printf (printf)
import System.Environment (getArgs)
import System.IO.Error (tryIOError)
import qualified Notification as N

downloadURL :: String -> IO String
downloadURL url = simpleHTTP (getRequest url) >>= getResponseBody

displayFeeds :: Int -> String -> IO ()
displayFeeds minL html =
    mapM_ (\(i,x) ->
        N.createFeedUpdate
            (createTitle i $ length feeds)
            (show x)
        ) feeds
    where
        createTitle = printf "Broadcastify Listener Update (%d of %d)"
        feeds =
            zip [(1::Int)..]
             . filter (\x -> listeners x > minL || isJust (info x))
             . Feed.createFromString
             $ html

runLoop :: Int -> Int -> IO ()
runLoop minL delayMin = do
    result <- tryIOError $ downloadURL "http://www.broadcastify.com/listen/top"
    case result of
        Left err -> void (N.createError $ show err)
        Right html -> displayFeeds minL html

    threadDelay $ 1000000 * 60 * delayMin
    runLoop minL delayMin

main :: IO ()
main = do
    args <- getArgs
    case args of
        [_, readMaybe -> Just minsToUpdate] | minsToUpdate < (5 :: Int) ->
            putStrLn "Update time must be >= 5 minutes"
        [readMaybe -> Just minListeners, readMaybe -> Just minsToUpdate] ->
            runLoop minListeners minsToUpdate
        _ -> do
            putStrLn "Usage: <minimum listeners> <update time in minutes>"
            runLoop 325 10
