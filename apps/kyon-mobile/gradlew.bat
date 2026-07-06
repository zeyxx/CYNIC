@echo off
@if "%DEBUG%" == "" @echo off
@rem Copyright 2015 the original authors.
@rem
goto find_java

:find_java
set JAVA_EXE=java
if not "%JAVA_HOME%" == "" set JAVA_EXE=%JAVA_HOME%\bin\java.exe

%JAVA_EXE% -version >NUL 2>&1
if %ERRORLEVEL% neq 0 goto fail

set APP_BASE_NAME=%~n0
set APP_HOME=%~dp0

set CLASSPATH=%APP_HOME%gradle\wrapper\gradle-wrapper.jar

%JAVA_EXE% %JAVA_OPTS% %GRADLE_OPTS% "-Dorg.gradle.appname=%APP_BASE_NAME%" -classpath "%CLASSPATH%" org.gradle.wrapper.GradleWrapperMain %*
exit /b %ERRORLEVEL%

:fail
echo Error: Could not find Java installation.
exit /b 1
