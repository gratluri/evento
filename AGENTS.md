Enterprise software implementations exceedingly rely on interactive subsystems to accomplish complex tasks. These subsystems could be monolithic or microservices-bases. The interactions could be syc or async (message oriented).

`evento` is an enterpise testing tool that will generate mockup messages to simuaate interactions in the context of enterprise testing. this could also substitue for mechanisam for load and performance testing.

the idea is to develop a highly flexible dsl for specifying the test. this includes a series of steps that has branches and loops to simulate complex interaction. Each interaction will have a message or request that is to be sent to the subsystem under test. The supported communication could be https, jms, kafka, grpc and flexible to add other protocols. the mocked message object could be json, xml, protobuf, sql query, cql query and could be extended to support different message formats. the mocked message data attributes could be specified randomly using faker library or could be obtained from a line item in flatfile or database (the mapping can be specified in the test script). the quality and quantity of the mocked messages should be configurable in the dsl language.

This whole project would be built in RUST and this project is open to adopt the best of the breed rust libraries and create facilities and libraries that are not yet available in the rust ecosystem.

The dsl should also have the ability to specify the result of the run in a stadnardized forat but that has ample flexibility. this tool should capture network, response,and be able to keep track of the business response like sucess, failure and other customer response based on the some parsed attributes that are specified in the dsl script.

Before, this implementation, I would like to develop this idea with details and rational that could be submitted to ycombinator
