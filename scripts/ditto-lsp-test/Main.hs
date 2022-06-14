{-# LANGUAGE BlockArguments #-}
{-# LANGUAGE ImportQualifiedPost #-}
{-# LANGUAGE LambdaCase #-}
{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE PackageImports #-}
{-# LANGUAGE NoImplicitPrelude #-}
{-# OPTIONS_GHC -Weverything -fno-warn-missing-import-lists -fno-warn-incomplete-uni-patterns -fno-warn-missing-safe-haskell-mode -fno-warn-all-missed-specialisations -fno-warn-unsafe #-}

{-
ghcid --command='stack repl cst-generator'
-}

module Main (main) where

import "lsp-test" Language.LSP.Test qualified as Lsp
import "lsp-types" Language.LSP.Types as LspTypes
import "base" System.Environment (getArgs)
import "base" System.Exit (die)
import "process" System.Process qualified as Process
import "base" Prelude

main :: IO ()
main =
  getArgs >>= \case
    [] -> die "missing arguments"
    [lspExe] -> test lspExe
    _ -> die "too many arguments"

test :: String -> IO ()
test lspExe = do
  testNotAProject lspExe

testNotAProject :: String -> IO ()
testNotAProject lspExe =
  runSession lspExe "crates/ditto-lsp/fixtures/not-a-project" do
    example <- Lsp.openDoc "Example.ditto" "ditto"

    -- semantic tokens
    Just LspTypes.SemanticTokens {} <- Lsp.getSemanticTokens example

    -- formatting
    Lsp.formatDoc
      example
      LspTypes.FormattingOptions
        { -- FIXME: these options are currently ignored
          LspTypes._tabSize = 4,
          LspTypes._insertSpaces = False,
          LspTypes._trimTrailingWhitespace = Just True,
          LspTypes._insertFinalNewline = Just True,
          LspTypes._trimFinalNewlines = Just True
        }

    pure ()

runSession :: String -> FilePath -> Lsp.Session a -> IO a
runSession lspExe rootDir session = do
  Process.withCreateProcess proc \(Just stdin) (Just stdout) _ _ -> do
    Lsp.runSessionWithHandles stdin stdout sessionConfig Lsp.fullCaps rootDir session
  where
    proc =
      (Process.proc lspExe [])
        { Process.std_in = Process.CreatePipe,
          Process.std_out = Process.CreatePipe,
          Process.cwd = Just rootDir
        }
    sessionConfig =
      Lsp.defaultConfig
        { Lsp.logStdErr = True,
          Lsp.logMessages = False
        }
