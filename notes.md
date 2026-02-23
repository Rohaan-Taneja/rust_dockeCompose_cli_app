for up , we will take docker compose file

first we need to see how to parse yaml file
we will take compose file , start/build image/file , connect ports , create dependecy graph and start combined docker

steps

## 📅 16th feb 2025

// baisclly parsing measn , converting it to a data structure which we can efficiently use to interact with docker api

1. how to parse yaml file => we used yaml serde compose crate and yaml serde to convert yaml file to compose(docker compose data structure ) ✅

## 📅 17th feb 2026

2)undertsnad it and how to communicate that to local docker and start , build files in process ❗❓
communication is done via bollard library , which internally calling docker provided api
to communicate and do all the start , stop , build , logs , statsu etc features

docker build . =? docker take this folder as context as a tar file and build image out of it , so to build an image , docker need context fo your project , it uses tar file

## 📅 18TH FEB 2026

converting our project folder into tar file , sending it to docker as stream of bites

## 📅 19th feb 2026

image is not building , will try to complete it
buidng image
image is building
starting container

## 📅 20th feb 2026

understand the tar file build and difference
understand which is better loading all files in ram/memory converted into tar and then sending ito the docker api at once
or creating a stream , and converting files to tar in chunks and send it to api in stream
at once approach will create ram/memory issue if file size is large
so sending in chunks is the good option

## 📅 21st feb 2026

understand the tar build logic in chunks

## 📅 22st feb 2026
building tar stream using os and excluding only target folder and .git , not extra function for now , may do it in the end ✅ 
1) image build/already_built_image => conatiner me start?  ✅ 
1.25) port binding  ✅ 


## 📅 23rd feb 2026
(it is just starting the image in conatiner , the logs and building is happening in logs only , so see that show logs in cli only ,erros and success )❓❓

docker compose has this folder build , the error showing is , why this folder is not starting , see error and resolve. ######

so now conatiner is just starting and function exit , starting of image runs in backend (no blocking of thread)

so if in docker compose file , we have serivce , which have healthcheck , then we will check for health == healthy 
else we will chec for only statsu == running 
but if we see with dependcy persepctive , we need to start a container completely , then only start the next container

parallel wala thing we will see 


1.5)  if image is not locally present , then get it from docker hub , then build it and conatinerize it 



2) then see how to build when git repo is provided ❓

3) then see how to parse the service in correct order so then loop to the array and start the containers in order , see parallel starteing of containers ❓❓

4) how all. containers are present in 1 container only ❓











now see whcih function call will-

1.  start the container from image provided
2.  start the container from build dockerfile 
3.  then see depedency graph se
4.  then see parallel starting of containers

for other we wil take contianer file 3) logs , status , stop , input will be container_id
