@echo off
:: Change directory to the directory of the batch script
cd /d "%~dp0"
:: Execute the executable from the subfolder
start "" "static_generator\target\release\static_generator.exe"
:: Pause the script to keep the command prompt window open
pause