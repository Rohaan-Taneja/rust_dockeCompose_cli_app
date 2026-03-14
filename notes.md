for up , we will take docker compose file

first we need to see how to parse yaml file
we will take compose file , start/build image/file , connect ports , create dependecy graph and start combined docker

steps

## 📅 14th feb 2025
started on 14th feb , sat

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
added the waiting for the to conatiner fully start feature , testing is left for current folder  ✅ 
so if in docker compose file , we have serivce , which have healthcheck , then we will check for health == healthy ✅ ✅ 
else we will chec for only statsu == running ✅ 


## 📅 24th feb 2026
so now conatiner is just starting and function exit , starting of image runs in backend (no blocking of thread) ✅ 
added support for image : local_image/docker_hub_image and build : .(already there)✅ ✅ 
1.5)  if image is not locally present , then get it from docker hub , then build it and conatinerize it ✅ ✅ 


## 📅 25th feb 2026
did not do anything 

## 📅 26th feb 2026
did not do anything


## 📅 27th feb 2026
did not do anything



## 📅 28th feb 2026
worked on flow of parsing compose struct to our vec and map
created flow and check_reorder function 


## 📅 1st March 2026
parse compose struct to our way , vec or services in depeds_on order and map of (service_name , service_details ) ✅ ✅ 


## 📅 2nd March 2026
resolved the current approach , debug it and it si genrating correct order✅
but this approach will not handle edge cases and will stuck in circular dependecies (updated the approach on 3rd march)✅ ✅ 

so we are trying to learn and implement the new approach 
create depedecy graph 

## 📅 3rd March 2026
created topology sort for string the services in corrent order and finding the cyclic dependency
creating a for loop to start all services on different ports (no waitng for service to start , no env , just images starting in conatiner)✅



## 📅 4th March 2026
thoda sa dekha 
did not do anything


## 📅 5th March 2026
if we are giving conatiner name , then why it is not showing up in the ui , conatiner name should be visible , it is showing up
correct the for loop , wait for the service to start then start the next service ❓❓


see how to start all services in 1 conatiner - ✅
    connect to same network✅
    add labels , then docker group them together✅
current_folder running ✅
fetching image from docker hub and running ✅ 
running local image ✅
port binding corrected  ✅


## 📅 6th March 2026
conatiner starting properly in porper order , waitng for the current once to start then next is starting  ✅
thoda sa dekha 
did not do anything


## 📅 7th March 2026
aaj , 6th and aaj ka kaam compesate krna hai 

conatiners label is now current dir name  ✅
modularize the file name form file_pah  ✅

delete containers + network properly of down command ✅


need to do solana , will continue on 10th 

## 📅 11th march
created the logger service , with colourfull println!s, ✅
now to give specific colour to each service , we though a pproach , will implemnt and comple it it tomorrow

## 📅 12th march
create random 4 alphabet string and attach to network/label name ❓❓ (same docker compose file , cannot run twice , so let it be like this only)
2) then create a logger service to show logs of service in colour and show service name also in that colour
3) then think how to status thing


## 📅 13th march
office
did not do anything

## 📅 14th march
understand colour of logger service working
create status cli api , show 
and then see whoch things are left and how to improve the project (currently its a eash project)


test starting and stopping again and then start with logs and status and enhancing the roject and code base


see if we can stop the conatiners (not delete) and then restart them ❓❓

this_project_network ❓❓ (we are taking network name ad this project name)
<!-- network name is hardcoded , make it dynamic -->


1) parallel wala thing we will see ❓❓

2) then see how to build when git repo is provided ❓

4) how all. containers are present in 1 group only ❓

5) env given ❓

6) network dynamic or hardcoded ❓

7) waiting for a service to start , then next service is starting , status and healthcheck ❓

8) we are deleteting the containers of down command , can we do stop and restarting ❓

8) health check not added , it is hardcoded , do it ❓
port is propvided , (but add error handling , see what if same port is given )❓








currently we are only supporting current folder build = . and not folder path
for image , we are supporting local image build and pull from docker and the build ❓



for other we wil take contianer file 3) logs , status , stop , input will be container_id
