
// we will create a struct to dispaly cli errors .



#[derive(Debug , Clone)]
pub struct CliErrors {
    message : String 

}

impl CliErrors {

    pub fn new( message : String)-> CliErrors {
        return CliErrors { message }
    }


    pub fn wrong_cli_name()-> CliErrors{
        return CliErrors { message: "wrong cli command , try with DockYard ...".to_string() }

    }

    pub fn wrong_cli_command() -> CliErrors{
        return CliErrors { message: "this cli command is not supported by dockyard , try Up , Down , Status , Logs".to_string() }
    }

    pub fn file_name_extraction_fail() -> CliErrors {
        return CliErrors { message: "we are getting while extracting file".to_string() }
    }

    pub fn wrong_docker_compose_file_name() -> CliErrors {
        return CliErrors { message: "the input docker compose file name is not correct , please check file name".to_string() }
    }

    pub fn wrong_file_path() -> CliErrors {
        return CliErrors { message: "wrong file path".to_string() }
    }


}






